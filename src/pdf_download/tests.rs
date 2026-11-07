use super::*;
use lopdf::{Document, Object, dictionary};

fn pdf(pages: usize, password: Option<&str>) -> Vec<u8> {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let kids: Vec<Object> = (0..pages)
        .map(|_| {
            doc.add_object(dictionary! {
                "Type" => "Page", "Parent" => pages_id,
                "MediaBox" => vec![0.into(), 0.into(), 300.into(), 400.into()],
            })
            .into()
        })
        .collect();
    doc.objects.insert(
        pages_id,
        dictionary! {
            "Type" => "Pages", "Kids" => kids, "Count" => pages as i64,
        }
        .into(),
    );
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog);
    if let Some(password) = password {
        doc.trailer.set(
            "ID",
            vec![
                Object::string_literal("fixture-id"),
                Object::string_literal("fixture-id"),
            ],
        );
        let state = lopdf::EncryptionState::try_from(lopdf::EncryptionVersion::V2 {
            document: &doc,
            owner_password: "owner",
            user_password: password,
            key_length: 128,
            permissions: lopdf::Permissions::default(),
        })
        .unwrap();
        doc.encrypt(&state).unwrap();
    }
    let mut data = Vec::new();
    doc.save_to(&mut data).unwrap();
    data
}

#[test]
fn accepts_detail_links_and_canonicalizes_only_the_standard_number() {
    let (code, url) = detail_link(" https://jjg.spc.org.cn:443/resmea/standard/detail.html?from=x&standno=JJF+1059.1-2012#preview ").unwrap();
    assert_eq!(code, "JJF 1059.1-2012");
    assert_eq!(
        url.as_str(),
        "https://jjg.spc.org.cn/resmea/standard/detail.html?standno=JJF+1059.1-2012"
    );
    for invalid in [
        "http://jjg.spc.org.cn/resmea/standard/detail.html?standno=JJF+1",
        "https://jjg.spc.org.cn.evil.example/resmea/standard/detail.html?standno=JJF+1",
        "https://user:password@jjg.spc.org.cn/resmea/standard/detail.html?standno=JJF+1",
        "https://jjg.spc.org.cn:444/resmea/standard/detail.html?standno=JJF+1",
        "https://jjg.spc.org.cn/resmea/standard/detail.html?standno=JJF+1&standno=JJF+2",
        "https://jjg.spc.org.cn/resmea/standard/detail.html?standno=%0d%0aHeader",
        "https://jjg.spc.org.cn/resmea/standard/detail.html?standno=%GG",
        "https://jjg.spc.org.cn/resmea/standard/detail.html?standno=+",
        "https://jjg.spc.org.cn/resmea/",
        "not a link",
    ] {
        assert!(detail_link(invalid).is_err(), "{invalid}");
    }
}

#[test]
fn searches_the_five_catalogs_by_name_and_normalized_number() {
    let catalog = standard_catalog();
    assert_eq!(catalog.len(), 2882);
    let mut codes = std::collections::HashSet::new();
    let categories: std::collections::HashSet<_> =
        catalog.iter().map(|row| row.category.as_str()).collect();
    assert_eq!(categories.len(), 5);
    for row in catalog {
        assert!(codes.insert(&row.code));
        assert!(!row.name.is_empty());
        assert_eq!(detail_link(&row.url).unwrap().0, row.code);
    }
    let rows = search_standards(" jjf1059.1－2012 ");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "测量不确定度评定与表示");
    assert!(!search_standards("压力 表").is_empty());
    assert!(search_standards("   ").is_empty());
    assert!(search_standards("no-such-standard-xyz").is_empty());
}

#[test]
fn parses_preview_parameters_and_title_without_accepting_header_injection() {
    assert_eq!(
        standard_name("<TITLE class='x'>温度 &amp; 压力 - 国家计量技术规范全文公开系统</TITLE>"),
        "温度 & 压力"
    );
    assert_eq!(standard_name("<title>访问错误</title>"), "");
    assert_eq!(
        preview_value(r#"var token = "abc\u002b123==";"#, "token").unwrap(),
        "abc+123=="
    );
    for page in [
        r#"var token = "";"#,
        r#"var token = "abc\r\nHeader";"#,
        "var token = undefined;",
        "changed website",
    ] {
        assert!(preview_value(page, "token").is_err());
    }
}

#[test]
fn filenames_are_valid_windows_names_and_do_not_cut_surrogate_pairs() {
    assert_eq!(
        default_filename("JJF 1/2020", "温度:测试?"),
        "JJF 1_2020 温度_测试_.pdf"
    );
    assert_eq!(default_filename("CON", ""), "_CON.pdf");
    assert_eq!(default_filename("LPT1", ""), "_LPT1.pdf");
    assert_eq!(default_filename("JJF 1", ""), "JJF 1.pdf");
    let name = default_filename("JJF", &"😀".repeat(200));
    assert!(name.trim_end_matches(".pdf").encode_utf16().count() <= 180);
}

#[test]
fn checks_actual_pages_empty_password_and_invalid_pdfs() {
    assert_eq!(validate_pdf(&pdf(3, None)).unwrap(), 3);
    assert_eq!(validate_pdf(&pdf(1, Some(""))).unwrap(), 1);
    assert!(
        validate_pdf(&pdf(1, Some("secret")))
            .unwrap_err()
            .contains("密码")
    );
    assert!(
        validate_pdf(&pdf(0, None))
            .unwrap_err()
            .contains("没有页面")
    );
    assert!(validate_pdf(b"<html>server error</html>").is_err());
    assert!(validate_pdf(b"%PDF-1.7\nnot a PDF").is_err());
}

#[test]
fn limits_response_bytes_and_rejects_http_errors_and_redirects() {
    assert_eq!(read_limited(&b"12345"[..], 5).unwrap(), b"12345");
    assert!(read_limited(&b"123456"[..], 5).is_err());
    for status in [302, 403, 429, 500] {
        let response = ureq::http::Response::builder()
            .status(status)
            .body(ureq::Body::builder().data("error"))
            .unwrap();
        let error = response_bytes(response, 10).unwrap_err();
        assert!(error.contains(&status.to_string()));
    }
}

#[test]
fn atomically_replaces_saved_pdf_and_cleans_up_after_a_failed_save() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("计量规范.pdf");
    std::fs::write(&path, "old").unwrap();
    let data = pdf(2, None);
    save_atomic(&path, &data).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), data);
    let blocked = directory.path().join("directory.pdf");
    std::fs::create_dir(&blocked).unwrap();
    assert!(save_atomic(&blocked, b"new").is_err());
    assert!(blocked.is_dir());
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 2);
}

#[test]
#[ignore = "访问公开规范网站，仅在手动验证下载时运行"]
fn live_download() {
    let result =
        download("https://jjg.spc.org.cn/resmea/standard/detail.html?standno=JJF+1059.1-2012")
            .unwrap();
    assert_eq!(result.code, "JJF 1059.1-2012");
    assert!(result.pages > 1);
    assert!(!result.name.is_empty());
    println!(
        "下载验证通过：{} {}，{} 页，{} 字节",
        result.code,
        result.name,
        result.pages,
        result.data.len()
    );
}
