use crate::calculators::common::{collapsible_note, render_error_message, render_metrics};
use crate::pdf_download::{self, Download};
use crate::{BLUE, BLUE_SOFT, INK, MUTED};
use eframe::egui::{self, RichText, TextEdit, Vec2};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;

fn action_button(label: &str, primary: bool) -> egui::Button<'_> {
    egui::Button::new(RichText::new(label).size(14.0).color(if primary {
        egui::Color32::WHITE
    } else {
        INK
    }))
    .fill(if primary { BLUE } else { egui::Color32::WHITE })
    .stroke(if primary {
        egui::Stroke::NONE
    } else {
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(207, 217, 232))
    })
    .corner_radius(6)
    .min_size(Vec2::new(112.0, 40.0))
}

#[derive(Default)]
pub(super) struct PdfDownloadPage {
    url: String,
    query: String,
    last_query: String,
    page: usize,
    job: Option<Receiver<Result<Download, String>>>,
    result: Option<Download>,
    message: String,
    error: Option<String>,
    choose_save: bool,
    confirm_download: bool,
    completion: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ToolboxApp;

    fn frame(app: &mut ToolboxApp, id: &'static str, width: f32) -> egui::FullOutput {
        let ctx = egui::Context::default();
        crate::fonts::configure_fonts(&ctx);
        crate::configure_style(&ctx);
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    [width, 820.0].into(),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.render_selected(ui, id));
            },
        )
    }

    #[test]
    fn acquisition_details_start_collapsed_and_can_be_toggled() {
        let ctx = egui::Context::default();
        crate::fonts::configure_fonts(&ctx);
        crate::configure_style(&ctx);
        let mut page = PdfDownloadPage::default();
        let render = |page: &mut PdfDownloadPage, events, time| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        [960.0, 1200.0].into(),
                    )),
                    events,
                    time: Some(time),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| page.render(ui));
                },
            )
        };
        let initial = render(&mut page, vec![], 0.0);
        let header = initial
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == "获取流程与接口说明" => {
                    Some(text.pos + Vec2::splat(4.0))
                }
                _ => None,
            })
            .expect("应显示折叠标题");
        let has_details =
            |output: &egui::FullOutput| {
                output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Text(text) if text.galley.text().starts_with("2. 请求在线预览页面")
        ))
            };
        assert!(!has_details(&initial), "说明应默认折叠");
        for (pressed, time) in [(true, 0.1), (false, 0.2)] {
            render(
                &mut page,
                vec![
                    egui::Event::PointerMoved(header),
                    egui::Event::PointerButton {
                        pos: header,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    },
                ],
                time,
            );
        }
        assert!(
            has_details(&render(&mut page, vec![], 0.5)),
            "点击后应展开说明"
        );
        for (pressed, time) in [(true, 0.6), (false, 0.7)] {
            render(
                &mut page,
                vec![
                    egui::Event::PointerMoved(header),
                    egui::Event::PointerButton {
                        pos: header,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    },
                ],
                time,
            );
        }
        assert!(
            !has_details(&render(&mut page, vec![], 1.0)),
            "再次点击后应折叠说明"
        );
    }

    #[test]
    fn navigation_keeps_running_download_and_pdf_links_are_not_saved() {
        let mut app = ToolboxApp::default();
        let _ = frame(&mut app, "metrology_pdf_download", 1220.0);
        app.pdf_download.url =
            "https://jjg.spc.org.cn/resmea/standard/detail.html?standno=JJF+1".into();
        let (sender, receiver) = mpsc::channel();
        app.pdf_download.job = Some(receiver);
        let _ = frame(&mut app, "unit_pressure", 620.0);
        let _ = frame(&mut app, "metrology_pdf_download", 620.0);
        assert!(app.pdf_download.busy());
        assert!(app.pdf_download.url.contains("JJF+1"));
        assert!(
            !app.history
                .values
                .keys()
                .any(|key| key.starts_with("metrology_pdf_download."))
        );
        app.pdf_download.clear();
        assert!(app.pdf_download.busy());
        sender.send(Err("模拟网络中断".into())).unwrap();
        app.pdf_download.poll(&egui::Context::default());
        assert!(!app.pdf_download.busy());
        assert_eq!(app.pdf_download.error.as_deref(), Some("模拟网络中断"));
        app.pdf_download.clear();
        assert!(app.pdf_download.url.is_empty());
        assert!(app.pdf_download.error.is_none());
    }

    #[test]
    fn clicking_a_search_result_confirms_the_selected_standard_before_download() {
        let ctx = egui::Context::default();
        crate::fonts::configure_fonts(&ctx);
        crate::configure_style(&ctx);
        let mut page = PdfDownloadPage {
            query: "jjf1059.1".into(),
            ..Default::default()
        };
        let render = |page: &mut PdfDownloadPage, events| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        [620.0, 820.0].into(),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| page.render(ui));
                },
            )
        };
        render(&mut page, vec![]);
        let first = render(&mut page, vec![]);
        let position = first
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == "下载 PDF" => {
                    Some(text.pos + Vec2::splat(4.0))
                }
                _ => None,
            })
            .unwrap_or_else(|| {
                let texts: Vec<_> = first
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) => Some((text.galley.text(), text.pos)),
                        _ => None,
                    })
                    .collect();
                panic!("搜索结果应有下载按钮：{texts:?}");
            });
        for pressed in [true, false] {
            render(
                &mut page,
                vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    },
                ],
            );
        }
        assert!(page.confirm_download);
        assert!(!page.busy());
        assert_eq!(
            page.url,
            "https://jjg.spc.org.cn/resmea/standard/detail.html?standno=JJF+1059.1-2012"
        );
        page.clear();
        assert!(page.query.is_empty());
        assert!(page.url.is_empty());
    }

    #[test]
    fn search_cards_share_size_and_show_full_names_at_different_widths() {
        for width in [375.0, 960.0] {
            let ctx = egui::Context::default();
            crate::fonts::configure_fonts(&ctx);
            crate::configure_style(&ctx);
            let mut page = PdfDownloadPage {
                query: "11".into(),
                ..Default::default()
            };
            let render = |page: &mut PdfDownloadPage| {
                ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            [width, 10000.0].into(),
                        )),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| page.render(ui));
                    },
                )
            };
            render(&mut page);
            let output = render(&mut page);
            let frames: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Rect(rect)
                        if rect.stroke.width > 0.0
                            && rect.stroke.color == BLUE_SOFT
                            && rect.rect.width() > width - 70.0 =>
                    {
                        Some(rect.rect)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(
                frames.len(),
                11,
                "应显示一个输入框和十个结果卡片：{frames:?}"
            );
            let cards = &frames[1..];
            for card in cards {
                assert!((card.width() - cards[0].width()).abs() < 0.5);
                assert!((card.height() - cards[0].height()).abs() < 0.5);
                assert!(card.max.x <= width, "结果卡片应位于窗口内");
            }
            let buttons: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Rect(rect)
                        if rect.fill == BLUE
                            && rect.rect.width() >= 100.0
                            && rect.rect.height() >= 35.0 =>
                    {
                        Some(rect.rect)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(
                buttons.len(),
                cards.len(),
                "每个结果卡片应有一个下载按钮：卡片 {cards:?}；按钮形状 {:?}",
                output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) if text.galley.text() == "下载 PDF" =>
                            Some(format!("text {:?}", text.pos)),
                        egui::Shape::Rect(rect) if rect.fill == BLUE =>
                            Some(format!("rect {:?}", rect.rect)),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            );
            for (button, card) in buttons.iter().zip(cards) {
                assert!(card.contains_rect(*button), "下载按钮应位于结果卡片内");
                assert!(
                    (button.min.x - buttons[0].min.x).abs() < 0.5,
                    "下载按钮应水平对齐：{buttons:?}"
                );
                assert!(
                    (button.center().y - card.center().y).abs() < 0.5,
                    "下载按钮应垂直居中：{button:?} / {card:?}"
                );
            }
            for ((standard, card), button) in pdf_download::search_standards("11")
                .iter()
                .take(10)
                .zip(cards)
                .zip(&buttons)
            {
                let text = output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) if text.galley.text() == standard.name => {
                            Some(text)
                        }
                        _ => None,
                    })
                    .expect("应显示完整规范名称");
                assert!(
                    card.contains_rect(egui::Rect::from_min_size(text.pos, text.galley.size())),
                    "规范名称超出卡片：{}",
                    standard.name
                );
                assert!(
                    text.pos.x + text.galley.size().x <= button.min.x,
                    "名称不应覆盖下载按钮"
                );
            }
            for (standard, card) in pdf_download::search_standards("11")
                .iter()
                .take(10)
                .zip(cards)
            {
                assert!(
                    output.shapes.iter().any(|shape| match &shape.shape {
                        egui::Shape::Text(text) if text.galley.text() == standard.category => card
                            .contains_rect(egui::Rect::from_min_size(text.pos, text.galley.size())),
                        _ => false,
                    }),
                    "每张卡片应单独显示规范类型：{}",
                    standard.code
                );
            }
        }
    }

    #[test]
    fn invalid_link_does_not_start_a_worker_or_discard_downloaded_data() {
        let mut page = PdfDownloadPage::default();
        page.result = Some(Download {
            code: "JJF 1".into(),
            name: "测试".into(),
            data: vec![1],
            pages: 1,
        });
        page.start();
        assert!(!page.busy());
        assert!(page.error.is_some());
        assert!(page.result.is_some());
    }

    #[test]
    fn link_input_has_a_single_full_height_border() {
        fn collect(shape: &egui::Shape, borders: &mut Vec<egui::Rect>) {
            match shape {
                egui::Shape::Rect(rect)
                    if rect.stroke.width > 0.0
                        && rect.rect.width() > 400.0
                        && (15.0..=50.0).contains(&rect.rect.height()) =>
                {
                    borders.push(rect.rect)
                }
                egui::Shape::Vec(shapes) => {
                    for shape in shapes {
                        collect(shape, borders);
                    }
                }
                _ => {}
            }
        }
        for width in [620.0, 1220.0] {
            let mut app = ToolboxApp::default();
            let output = frame(&mut app, "metrology_pdf_download", width);
            let mut borders = Vec::new();
            for shape in &output.shapes {
                collect(&shape.shape, &mut borders);
            }
            assert_eq!(
                borders.len(),
                1,
                "不能在文字内侧再绘制一层边框：{borders:?}"
            );
            assert!(borders[0].height() >= 40.0);
        }
    }

    fn dialog_frame(
        page: &mut PdfDownloadPage,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    [620.0, 520.0].into(),
                )),
                events,
                ..Default::default()
            },
            |ctx| page.render_dialogs(ctx),
        )
    }

    fn dismiss_dialog(page: &mut PdfDownloadPage, ctx: &egui::Context) {
        dialog_frame(page, ctx, vec![]);
        dialog_frame(
            page,
            ctx,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Default::default(),
            }],
        );
    }

    #[test]
    fn warning_can_be_cancelled_without_starting_network_requests() {
        let mut page = PdfDownloadPage::default();
        page.url = "https://jjg.spc.org.cn/resmea/standard/detail.html?standno=JJF+1".into();
        page.request_download();
        assert!(page.confirm_download);
        assert!(!page.busy());
        dismiss_dialog(&mut page, &egui::Context::default());
        assert!(!page.confirm_download);
        assert!(!page.busy());
        page.url.clear();
        page.request_download();
        assert!(!page.confirm_download);
        assert!(page.error.is_some());
    }

    #[test]
    fn dialogs_fit_the_small_window_and_keep_full_size_action_buttons() {
        fn collect(shape: &egui::Shape, rects: &mut Vec<egui::Rect>) {
            match shape {
                egui::Shape::Rect(rect) => rects.push(rect.rect),
                egui::Shape::Vec(shapes) => {
                    for shape in shapes {
                        collect(shape, rects);
                    }
                }
                _ => {}
            }
        }
        for warning in [true, false] {
            let ctx = egui::Context::default();
            crate::fonts::configure_fonts(&ctx);
            let mut page = PdfDownloadPage::default();
            page.confirm_download = warning;
            if !warning {
                page.completion =
                    Some("JJF 1 测试规范\nPDF 页数：2 页\n\n已保存至：C:\\规范.pdf".into());
            }
            let _ = dialog_frame(&mut page, &ctx, vec![]);
            let output = dialog_frame(&mut page, &ctx, vec![]);
            let mut rects = Vec::new();
            for shape in &output.shapes {
                collect(&shape.shape, &mut rects);
            }
            let panel = rects
                .iter()
                .find(|r| (480.0..510.0).contains(&r.width()))
                .unwrap();
            assert!(
                panel.height() < 330.0 && panel.top() >= 0.0 && panel.bottom() <= 520.0,
                "{panel:?}"
            );
            assert_eq!(
                rects
                    .iter()
                    .filter(|r| r.width() >= 112.0 && (39.0..42.0).contains(&r.height()))
                    .count(),
                if warning { 2 } else { 1 }
            );
        }
    }

    #[test]
    fn success_dialog_appears_only_after_the_file_is_saved() {
        let directory = tempfile::tempdir().unwrap();
        let mut page = PdfDownloadPage::default();
        page.result = Some(Download {
            code: "JJF 1".into(),
            name: "测试规范".into(),
            data: b"downloaded bytes".to_vec(),
            pages: 1,
        });
        page.save_to(directory.path());
        assert!(page.error.is_some());
        assert!(page.completion.is_none());
        let path = directory.path().join("计量规范.pdf");
        page.save_to(&path);
        assert_eq!(std::fs::read(&path).unwrap(), b"downloaded bytes");
        assert!(page.error.is_none());
        let message = page.completion.as_ref().unwrap();
        assert!(
            message.contains("测试规范")
                && message.contains("1 页")
                && message.contains("计量规范.pdf")
        );
        assert!(message.contains("只有一页"));
        dismiss_dialog(&mut page, &egui::Context::default());
        assert!(page.completion.is_none());
        assert!(path.exists());
    }
}

impl PdfDownloadPage {
    pub(super) fn busy(&self) -> bool {
        self.job.is_some()
    }

    pub(super) fn clear(&mut self) {
        if !self.busy() {
            *self = Self::default();
        }
    }

    pub(super) fn poll(&mut self, ctx: &egui::Context) {
        let Some(job) = &self.job else { return };
        match job.try_recv() {
            Ok(result) => {
                self.job = None;
                match result {
                    Ok(download) => {
                        self.message = "PDF 已获取并通过检查，请选择保存位置。".into();
                        self.result = Some(download);
                        self.choose_save = true;
                    }
                    Err(error) => self.error = Some(error),
                }
            }
            Err(TryRecvError::Disconnected) => {
                self.job = None;
                self.error = Some("下载线程异常退出，请重试。".into());
            }
            Err(TryRecvError::Empty) => ctx.request_repaint_after(Duration::from_millis(100)),
        }
    }

    pub(super) fn render(&mut self, ui: &mut egui::Ui) {
        ui.set_max_width(ui.available_width().min(960.0));
        egui::Frame::new()
            .fill(egui::Color32::WHITE)
            .stroke(egui::Stroke::new(1.0_f32, BLUE_SOFT))
            .corner_radius(8)
            .inner_margin(18)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(12.0, 10.0);
                ui.label(
                    RichText::new("规范名称或编号")
                        .size(15.0)
                        .strong()
                        .color(INK),
                );
                ui.label(
                    RichText::new("输入名称关键词或规范编号筛选，在结果中选择需要下载的规范。")
                        .size(12.0)
                        .color(MUTED),
                );
                ui.add_enabled_ui(!self.busy(), |ui| {
                    let mut field = egui::Frame::new()
                        .fill(egui::Color32::WHITE)
                        .stroke(egui::Stroke::new(
                            1.0_f32,
                            egui::Color32::from_rgb(169, 189, 219),
                        ))
                        .corner_radius(6)
                        .inner_margin(egui::Margin::symmetric(12, 10))
                        .begin(ui);
                    let response = field.content_ui.add_sized(
                        [field.content_ui.available_width(), 20.0],
                        TextEdit::singleline(&mut self.query)
                            .frame(false)
                            .margin(0)
                            .vertical_align(egui::Align::Center)
                            .font(egui::FontId::proportional(14.0)),
                    );
                    if response.has_focus() {
                        field.frame.stroke.color = BLUE;
                    }
                    field.end(ui);
                });
                ui.label(
                    RichText::new("数据来源：https://jjg.spc.org.cn/（截至 2026-11 的规范目录）")
                        .size(12.0)
                        .color(MUTED),
                );
                ui.horizontal_wrapped(|ui| {
                    ui.hyperlink_to("打开规范网站", "https://jjg.spc.org.cn/resmea/");
                    if self.busy() {
                        ui.spinner();
                        ui.label("正在获取规范名称并检查 PDF…");
                    }
                });
            });
        self.render_search_results(ui);
        if let Some(download) = &self.result {
            render_metrics(
                ui,
                "下载结果",
                &[
                    ("规范编号", download.code.clone()),
                    (
                        "规范名称",
                        if download.name.is_empty() {
                            "未获取，请在保存时补充".into()
                        } else {
                            download.name.clone()
                        },
                    ),
                    ("PDF 页数", format!("{} 页", download.pages)),
                ],
            );
            if download.pages == 1 {
                ui.label(
                    RichText::new("PDF 只有一页，请打开文件确认是否为规范正文。").color(crate::RED),
                );
            }
            if ui.add(action_button("保存 PDF…", false)).clicked() {
                self.choose_save = true;
            }
        }
        if let Some(error) = &self.error {
            render_error_message(ui, error);
        }
        if !self.message.is_empty() {
            ui.label(RichText::new(&self.message).color(MUTED));
        }
        ui.add_space(12.0);
        collapsible_note(
            ui,
            "pdf_acquisition_details",
            "获取流程与接口说明",
            |ui| {
                ui.label(
                    RichText::new(
                        "1. 按名称或编号搜索内置目录，选择规范后使用对应详情页链接获取 PDF。",
                    )
                    .size(13.0)
                    .color(INK),
                );
                ui.label(RichText::new("2. 请求在线预览页面，提交 a100（规范编号）和 ismobile=1，取得 token 与 enc 参数。").size(13.0).color(INK));
                ui.add(
                    egui::Label::new(
                        RichText::new("POST https://jjg.spc.org.cn/resmea/view/stdonline")
                            .monospace()
                            .size(12.0)
                            .color(MUTED),
                    )
                    .wrap(),
                );
                ui.label(RichText::new("3. 沿用同一会话的 Cookie，以 token 作为查询参数、enc 作为 myfoxit 请求头，获取预览 PDF。").size(13.0).color(INK));
                ui.add(
                    egui::Label::new(
                        RichText::new(
                            "GET https://jjg.spc.org.cn/resmea/view/onlinereading?token=…",
                        )
                        .monospace()
                        .size(12.0)
                        .color(MUTED),
                    )
                    .wrap(),
                );
                ui.label(RichText::new("4. 检查 PDF 格式、已知错误提示文件及实际页数，再按所选位置保存网站返回的原始 PDF。").size(13.0).color(INK));
                ui.separator();
                ui.label(RichText::new("以上为网站在线预览使用的接口，网站调整或限流时可能无法获取。仅支持本站 HTTPS 详情页，单个 PDF 上限 100 MB；请按网站的电子文本使用说明使用文件。").size(12.0).color(MUTED));
            },
        );
        if self.choose_save {
            self.choose_save = false;
            self.save();
        }
        self.render_dialogs(ui.ctx());
    }

    fn render_search_results(&mut self, ui: &mut egui::Ui) {
        if self.query != self.last_query {
            self.page = 0;
            self.last_query = self.query.clone();
        }
        if self.query.trim().is_empty() {
            return;
        }
        let rows = pdf_download::search_standards(&self.query);
        let pages = rows.len().div_ceil(10).max(1);
        self.page = self.page.min(pages - 1);
        ui.add_space(12.0);
        ui.label(
            RichText::new(format!("找到 {} 条规范", rows.len()))
                .strong()
                .color(INK),
        );
        if rows.is_empty() {
            ui.label("未找到匹配的规范，请尝试其他名称或编号。");
        }
        let start = self.page * 10;
        let visible = &rows[start..rows.len().min(start + 10)];
        let card_width = (ui.available_width() - 28.0).max(0.0);
        let text_width = (card_width - 140.0).max(140.0);
        let row_height = visible
            .iter()
            .map(|standard| {
                [
                    (standard.code.clone(), 14.0),
                    (standard.name.clone(), 15.0),
                    (standard.category.clone(), 12.0),
                    (
                        format!(
                            "状态：{}　发布：{}　实施：{}",
                            standard.status,
                            standard.published,
                            if standard.effective.is_empty() {
                                "未提供"
                            } else {
                                &standard.effective
                            }
                        ),
                        12.0,
                    ),
                ]
                .into_iter()
                .map(|(text, size)| {
                    ui.painter()
                        .layout(text, egui::FontId::proportional(size), INK, text_width)
                        .size()
                        .y
                        .max(ui.spacing().interact_size.y)
                })
                .sum::<f32>()
                    + ui.spacing().item_spacing.y * 3.0
                    + 8.0
            })
            .fold(40.0_f32, f32::max);
        for standard in visible {
            egui::Frame::new()
                .fill(egui::Color32::WHITE)
                .stroke(egui::Stroke::new(1.0_f32, BLUE_SOFT))
                .corner_radius(8)
                .inner_margin(14)
                .show(ui, |ui| {
                    ui.set_min_width(card_width);
                    ui.set_min_height(row_height);
                    ui.allocate_ui_with_layout(
                        Vec2::new(card_width, row_height),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if ui
                                .add_enabled(!self.busy(), action_button("下载 PDF", true))
                                .clicked()
                            {
                                self.url = standard.url.clone();
                                self.request_download();
                            }
                            ui.allocate_ui_with_layout(
                                Vec2::new(text_width, row_height),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    ui.set_max_width(text_width);
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(&standard.code).strong().color(BLUE),
                                        )
                                        .wrap(),
                                    );
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(&standard.name).size(15.0).color(INK),
                                        )
                                        .wrap(),
                                    );
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(&standard.category)
                                                .size(12.0)
                                                .color(MUTED),
                                        )
                                        .wrap(),
                                    );
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(format!(
                                                "状态：{}　发布：{}　实施：{}",
                                                standard.status,
                                                standard.published,
                                                if standard.effective.is_empty() {
                                                    "未提供"
                                                } else {
                                                    &standard.effective
                                                }
                                            ))
                                            .size(12.0)
                                            .color(
                                                if matches!(
                                                    standard.status.as_str(),
                                                    "被代替" | "废止"
                                                ) {
                                                    crate::RED
                                                } else {
                                                    MUTED
                                                },
                                            ),
                                        )
                                        .wrap(),
                                    );
                                },
                            );
                        },
                    );
                });
            ui.add_space(6.0);
        }
        if !rows.is_empty() {
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!self.busy() && self.page > 0, egui::Button::new("上一页"))
                    .clicked()
                {
                    self.page -= 1;
                }
                ui.label(format!("第 {} / {} 页", self.page + 1, pages));
                if ui
                    .add_enabled(
                        !self.busy() && self.page + 1 < pages,
                        egui::Button::new("下一页"),
                    )
                    .clicked()
                {
                    self.page += 1;
                }
            });
        }
        ui.label(
            RichText::new("规范状态与实施日期请以规范网站为准。")
                .size(12.0)
                .color(MUTED),
        );
        ui.add_space(12.0);
    }

    fn request_download(&mut self) {
        match pdf_download::detail_link(&self.url) {
            Ok(_) => {
                self.error = None;
                self.confirm_download = true;
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn render_dialogs(&mut self, ctx: &egui::Context) {
        let frame = egui::Frame::popup(&ctx.style())
            .fill(egui::Color32::WHITE)
            .stroke(egui::Stroke::new(
                1.0_f32,
                egui::Color32::from_rgb(220, 227, 237),
            ))
            .corner_radius(10)
            .inner_margin(24);
        if self.confirm_download {
            let dialog = egui::Modal::new(egui::Id::new("pdf_download_warning")).frame(frame).show(ctx, |ui| {
                ui.set_width(440.0);
                ui.spacing_mut().item_spacing = Vec2::new(12.0, 12.0);
                ui.label(RichText::new("下载提示").size(19.0).strong().color(INK));
                if let Some(standard) = pdf_download::standard_catalog().iter().find(|row| row.url == self.url) {
                    ui.label(RichText::new(format!("{} {}", standard.code, standard.name)).strong().color(INK));
                    ui.label(RichText::new(format!("{} · 实施日期：{}", standard.status,
                        if standard.effective.is_empty() { "未提供" } else { &standard.effective })).size(12.0).color(MUTED));
                }
                ui.add_space(8.0);
                ui.label(RichText::new("短时间内频繁下载可能触发网站限流（HTTP 429 Too Many Requests），请适当控制下载频率。").size(14.0).color(INK));
                ui.label(RichText::new("请按网站的电子文本使用说明使用下载文件。").size(13.0).color(MUTED));
                ui.add_space(12.0);
                ui.allocate_ui_with_layout(Vec2::new(ui.available_width(), 40.0), egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(action_button("知道了，继续下载", true)).clicked() {
                        self.confirm_download = false;
                        self.start();
                    }
                    if ui.add(action_button("取消", false)).clicked() {
                        self.confirm_download = false;
                    }
                });
            });
            if dialog.should_close() {
                self.confirm_download = false;
            }
        }
        if let Some(message) = self.completion.clone() {
            let dialog = egui::Modal::new(egui::Id::new("pdf_download_completed"))
                .frame(frame)
                .show(ctx, |ui| {
                    ui.set_width(440.0);
                    ui.spacing_mut().item_spacing = Vec2::new(12.0, 12.0);
                    ui.label(RichText::new("下载成功").size(19.0).strong().color(INK));
                    ui.add_space(8.0);
                    ui.label(RichText::new(message).size(14.0).color(INK));
                    ui.add_space(12.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(ui.available_width(), 40.0),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if ui.add(action_button("确定", true)).clicked() {
                                self.completion = None;
                            }
                        },
                    );
                });
            if dialog.should_close() {
                self.completion = None;
            }
        }
    }

    fn start(&mut self) {
        self.error = None;
        self.message.clear();
        if let Err(error) = pdf_download::detail_link(&self.url) {
            self.error = Some(error);
            return;
        }
        self.result = None;
        let url = self.url.clone();
        let (sender, receiver) = mpsc::channel();
        match std::thread::Builder::new()
            .name("metrology-pdf".into())
            .spawn(move || {
                let _ = sender.send(pdf_download::download(&url));
            }) {
            Ok(_) => self.job = Some(receiver),
            Err(error) => self.error = Some(format!("无法启动下载：{error}")),
        }
    }

    fn save(&mut self) {
        let Some(download) = &self.result else { return };
        let Some(path) = rfd::FileDialog::new()
            .set_title("保存计量规范 PDF")
            .add_filter("PDF 文件", &["pdf"])
            .set_file_name(pdf_download::default_filename(
                &download.code,
                &download.name,
            ))
            .save_file()
        else {
            self.message = "已取消保存，可以点击“保存 PDF”重新选择位置。".into();
            return;
        };
        self.save_to(&path);
    }

    fn save_to(&mut self, path: &std::path::Path) {
        let Some(download) = &self.result else { return };
        match pdf_download::save_atomic(path, &download.data) {
            Ok(()) => {
                self.error = None;
                self.message = format!("已保存：{}", path.display());
                self.completion = Some(format!(
                    "{} {}\nPDF 页数：{} 页\n\n已保存至：{}{}",
                    download.code,
                    download.name,
                    download.pages,
                    path.display(),
                    if download.pages == 1 {
                        "\n\nPDF 只有一页，请打开文件确认是否为规范正文。"
                    } else {
                        ""
                    }
                ));
            }
            Err(error) => self.error = Some(error),
        }
    }
}
