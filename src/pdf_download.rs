//! 复用 JJF-PDF-Demo 的站点流程；只接收本站详情页，保存网站返回的原始字节。
use regex::Regex;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;
use url::Url;

const ROOT: &str = "https://jjg.spc.org.cn";
const MAX_PDF_BYTES: usize = 100 * 1024 * 1024;
const MAX_HTML_BYTES: usize = 1024 * 1024;
const ERROR_PDF_HASH: &str = "f487c0adbe09950cd6effdbca2af0b9d59b4c53ed7adbf5e58cfff694b3e2ed8";
const AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36 Edg/130.0.0.0";

pub(crate) struct Download {
    pub(crate) code: String,
    pub(crate) name: String,
    pub(crate) data: Vec<u8>,
    pub(crate) pages: usize,
}

pub(crate) struct Standard {
    pub(crate) code: String,
    pub(crate) name: String,
    pub(crate) category: String,
    pub(crate) status: String,
    pub(crate) published: String,
    pub(crate) effective: String,
    pub(crate) url: String,
    search_code: String,
    search_name: String,
}

fn search_key(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(char::to_lowercase)
        .map(|c| {
            if matches!(c, '－' | '—' | '–') {
                '-'
            } else {
                c
            }
        })
        .collect()
}

pub(crate) fn standard_catalog() -> &'static [Standard] {
    static CATALOG: OnceLock<Vec<Standard>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let data: serde_json::Value =
            serde_json::from_str(include_str!("../data/standards/catalog.json"))
                .expect("内置规范目录 JSON 无效");
        data["standards"]
            .as_array()
            .expect("内置规范目录缺少清单")
            .iter()
            .map(|row| {
                let field = |key| row[key].as_str().expect("内置规范字段无效").to_owned();
                let code = field("code");
                let name = field("name");
                Standard {
                    search_code: search_key(&code),
                    search_name: search_key(&name),
                    code,
                    name,
                    category: field("category"),
                    status: field("status"),
                    published: field("published"),
                    effective: field("effective"),
                    url: field("url"),
                }
            })
            .collect()
    })
}

pub(crate) fn search_standards(query: &str) -> Vec<&'static Standard> {
    let key = search_key(query);
    if key.is_empty() {
        return Vec::new();
    }
    standard_catalog()
        .iter()
        .filter(|row| row.search_code.contains(&key) || row.search_name.contains(&key))
        .collect()
}

pub(crate) fn detail_link(input: &str) -> Result<(String, Url), String> {
    let input = input.trim();
    if input.chars().any(char::is_control) {
        return Err("链接中包含无效字符。".into());
    }
    let url = Url::parse(input).map_err(|_| "请输入有效的规范详情页链接。")?;
    if url.scheme() != "https"
        || url.host_str() != Some("jjg.spc.org.cn")
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/resmea/standard/detail.html"
    {
        return Err("请输入国家计量技术规范全文公开系统的 HTTPS 规范详情页链接。".into());
    }
    let codes: Vec<_> = url
        .query_pairs()
        .filter(|(key, _)| key == "standno")
        .collect();
    if codes.len() != 1 {
        return Err("链接中没有唯一有效的规范编号。".into());
    }
    let raw = &codes[0].1;
    let code = raw.trim();
    if code.is_empty()
        || raw.len() > 100
        || !raw
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"/ ._-".contains(&c))
    {
        return Err("链接中没有有效的规范编号。".into());
    }
    // 丢弃页面附加参数和片段，后续请求始终使用固定站点、路径和规范编号。
    let mut canonical = Url::parse(&format!("{ROOT}/resmea/standard/detail.html")).unwrap();
    canonical.query_pairs_mut().append_pair("standno", code);
    Ok((code.to_owned(), canonical))
}

fn standard_name(page: &str) -> String {
    let pattern = Regex::new(r"(?is)<title\b[^>]*>(.*?)</title\s*>").unwrap();
    let Some(title) = pattern.captures(page) else {
        return String::new();
    };
    let title = html_escape::decode_html_entities(&title[1]);
    title
        .trim()
        .strip_suffix(" - 国家计量技术规范全文公开系统")
        .unwrap_or_default()
        .trim()
        .to_owned()
}

fn preview_value(page: &str, name: &str) -> Result<String, String> {
    let pattern = Regex::new(&format!(
        r#"\bvar\s+{}\s*=\s*("(?:\\.|[^"\\])*")"#,
        regex::escape(name)
    ))
    .unwrap();
    let value = pattern
        .captures(page)
        .and_then(|m| serde_json::from_str::<String>(&m[1]).ok())
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 4096
                && value
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"+/=".contains(&c))
        });
    value.ok_or_else(|| "预览页面缺少有效参数，网站可能已更新。".into())
}

pub(crate) fn default_filename(code: &str, name: &str) -> String {
    let label = format!("{code} {name}");
    let mut length = 0;
    let mut label: String = label
        .trim()
        .chars()
        .map(|c| {
            if c.is_control() || "\\/:*?\"<>|".contains(c) {
                '_'
            } else {
                c
            }
        })
        .take_while(|c| {
            length += c.len_utf16();
            length <= 180
        })
        .collect();
    label = label.trim_end_matches([' ', '.']).to_owned();
    let stem = label
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'))
    {
        label.insert(0, '_');
    }
    format!("{label}.pdf")
}

pub(crate) fn validate_pdf(data: &[u8]) -> Result<usize, String> {
    if data.len() > MAX_PDF_BYTES {
        return Err("文件超过 100 MB 的大小限制。".into());
    }
    if !data.starts_with(b"%PDF-") {
        return Err("网站没有返回有效的 PDF。".into());
    }
    let hash: String = Sha256::digest(data)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if hash == ERROR_PDF_HASH {
        return Err("网站返回了在线阅读加载失败的提示文件。".into());
    }
    let document = lopdf::Document::load_mem_with_options(
        data,
        lopdf::LoadOptions::with_max_decompressed_size(MAX_PDF_BYTES),
    )
    .map_err(|error| match error {
        lopdf::Error::InvalidPassword | lopdf::Error::Decryption(_) => {
            "PDF 需要密码或使用了不支持的加密方式，无法检查。".to_owned()
        }
        _ => "网站返回的 PDF 无法解析。".to_owned(),
    })?;
    if document.is_encrypted() {
        return Err("PDF 需要密码，无法检查。".into());
    }
    let pages = document.get_pages().len();
    if pages == 0 {
        return Err("PDF 没有页面。".into());
    }
    Ok(pages)
}

fn response_bytes(
    mut response: ureq::http::Response<ureq::Body>,
    limit: usize,
) -> Result<Vec<u8>, String> {
    let status = response.status().as_u16();
    if status == 429 {
        return Err("请求过于频繁（HTTP 429），请稍后再试。".into());
    }
    if !(200..300).contains(&status) {
        return Err(format!("网站返回 HTTP {status}。"));
    }
    read_limited(response.body_mut().as_reader(), limit)
}

fn read_limited(reader: impl Read, limit: usize) -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    reader
        .take(limit as u64 + 1)
        .read_to_end(&mut data)
        .map_err(|error| format!("读取网站响应失败：{error}"))?;
    if data.len() > limit {
        return Err("网站返回的数据超过大小限制（PDF 上限 100 MB）。".into());
    }
    Ok(data)
}

fn network_error(error: ureq::Error) -> String {
    format!("连接网站失败，请检查网络或稍后重试：{error}")
}

pub(crate) fn download(input: &str) -> Result<Download, String> {
    let (code, url) = detail_link(input)?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .user_agent(AGENT)
        .max_redirects(0)
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(30)))
        .build()
        .into();
    let name = agent
        .get(url.as_str())
        .header("Referer", ROOT)
        .call()
        .ok()
        .and_then(|response| response_bytes(response, MAX_HTML_BYTES).ok())
        .map(|data| standard_name(&String::from_utf8_lossy(&data)))
        .unwrap_or_default();
    let preview_url = format!("{ROOT}/resmea/view/stdonline");
    let preview = agent
        .post(&preview_url)
        .header("Referer", url.as_str())
        .send_form([("a100", code.as_str()), ("ismobile", "1")])
        .map_err(network_error)?;
    let preview = response_bytes(preview, MAX_HTML_BYTES)?;
    let page = String::from_utf8_lossy(&preview);
    let token = preview_value(&page, "token")?;
    let enc = preview_value(&page, "enc")?;
    let mut pdf_url = Url::parse(&format!("{ROOT}/resmea/view/onlinereading")).unwrap();
    pdf_url.query_pairs_mut().append_pair("token", &token);
    let response = agent
        .get(pdf_url.as_str())
        .header("Referer", &preview_url)
        .header("myfoxit", &enc)
        .header("X-Requested-With", "XMLHttpRequest")
        .header("Accept", "*/*")
        .call()
        .map_err(network_error)?;
    let data = response_bytes(response, MAX_PDF_BYTES)?;
    let pages = validate_pdf(&data)?;
    Ok(Download {
        code,
        name,
        data,
        pages,
    })
}

pub(crate) fn save_atomic(path: &Path, data: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temp =
        tempfile::NamedTempFile::new_in(parent).map_err(|e| format!("无法创建临时文件：{e}"))?;
    temp.write_all(data)
        .and_then(|_| temp.flush())
        .and_then(|_| temp.as_file().sync_all())
        .map_err(|e| format!("写入文件失败：{e}"))?;
    temp.persist(path)
        .map_err(|e| format!("保存失败，请确认目标文件未被占用：{}", e.error))?;
    Ok(())
}
