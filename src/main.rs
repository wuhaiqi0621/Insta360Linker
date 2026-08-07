mod adapters;
mod core;
mod profiles;

use adapters::camera_osc::OscClient;
use adapters::watermark::WatermarkOptions;
use eframe::egui;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Camera,
    Mic,
    Watermark,
    Profiles,
}

struct AppState {
    page: Page,
    jobs: core::JobRunner,
    host: String,
    port: String,
    command_name: String,
    command_params: String,
    file_url: String,
    camera_output: String,
    ble_output: String,
    ble_address: String,
    gatt_uuid: String,
    gatt_hex: String,
    input_media: String,
    output_media: String,
    wm_position: String,
    profiles_text: String,
    log: Vec<String>,
    media_files: Vec<adapters::luna_local::LunaFile>,
    luna_stop: Option<mpsc::Sender<()>>,
    luna_session_active: bool,
}

impl Default for AppState {
    fn default() -> Self {
        let profiles_text = profiles::summaries()
            .map(|p| serde_json::to_string_pretty(&p).unwrap_or_default())
            .unwrap_or_else(|e| format!("设备档案加载失败：{e}"));
        Self {
            page: Page::Camera,
            jobs: core::JobRunner::new(),
            host: "192.168.42.1".to_string(),
            port: "80".to_string(),
            command_name: "camera.getOptions".to_string(),
            command_params: r#"{"optionNames":["captureMode","_videoType","remainingSpace","_batteryCapacity"]}"#.to_string(),
            file_url: String::new(),
            camera_output: String::new(),
            ble_output: String::new(),
            ble_address: String::new(),
            gatt_uuid: String::new(),
            gatt_hex: String::new(),
            input_media: String::new(),
            output_media: String::new(),
            wm_position: "bottom-center".to_string(),
            profiles_text,
            log: vec!["就绪".to_string()],
            media_files: Vec::new(),
            luna_stop: None,
            luna_session_active: false,
        }
    }
}

impl eframe::App for AppState {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.apply_theme(ui.ctx());
        for result in self.jobs.drain() {
            match result {
                Ok(text) => {
                    if text.starts_with("[BLE]") {
                        self.ble_output = text;
                    } else if let Ok(files) =
                        serde_json::from_str::<Vec<adapters::luna_local::LunaFile>>(&text)
                    {
                        self.media_files = files;
                        self.camera_output = text.clone();
                        self.log
                            .push(format!("已读取 {} 个素材", self.media_files.len()));
                    } else {
                        self.camera_output = text.clone();
                        self.log.push("完成".to_string());
                    }
                }
                Err(err) => self.log.push(format!("失败：{err}")),
            }
        }

        ui.vertical(|ui| {
            self.hero(ui);
            self.top_nav(ui);
            ui.separator();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    match self.page {
                        Page::Camera => self.camera_page(ui),
                        Page::Mic => self.mic_page(ui),
                        Page::Watermark => self.watermark_page(ui),
                        Page::Profiles => self.profiles_page(ui),
                    }
                });
            ui.separator();
            self.log_panel(ui);
        });
    }
}

impl AppState {
    fn apply_theme(&self, ctx: &egui::Context) {
        let mut visuals = egui::Visuals::light();
        visuals.panel_fill = egui::Color32::from_rgb(243, 246, 250);
        visuals.window_fill = egui::Color32::from_rgb(248, 250, 253);
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(0, 95, 184);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(232, 240, 252);
        visuals.selection.bg_fill = egui::Color32::from_rgb(0, 95, 184);
        ctx.set_visuals(visuals);
    }

    fn hero(&self, ui: &mut egui::Ui) {
        egui::Frame::new()
            .fill(egui::Color32::from_rgb(235, 243, 255))
            .corner_radius(10.0)
            .inner_margin(egui::Margin::symmetric(18, 14))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.heading("Luna 控制台");
                        ui.label(
                            egui::RichText::new("连接 Luna Ultra、管理素材、添加水印")
                                .color(egui::Color32::from_rgb(58, 69, 86)),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("Rust 原生应用")
                                .color(egui::Color32::from_rgb(0, 95, 184))
                                .strong(),
                        );
                    });
                });
            });
        ui.add_space(10.0);
    }

    fn top_nav(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            self.nav_button(ui, Page::Camera, "相机", "Luna Ultra");
            self.nav_button(ui, Page::Mic, "麦克风", "Mic Pro");
            self.nav_button(ui, Page::Watermark, "水印", "导出叠加");
            self.nav_button(ui, Page::Profiles, "设备档案", "协议配置");
            ui.separator();
            let status = if self.luna_session_active {
                "会话保持中"
            } else {
                "会话未打开"
            };
            ui.label(
                egui::RichText::new(format!("Luna {status}"))
                    .small()
                    .color(egui::Color32::from_rgb(92, 99, 112)),
            );
        });
        ui.add_space(8.0);
    }

    fn nav_button(&mut self, ui: &mut egui::Ui, page: Page, title: &str, subtitle: &str) {
        let selected = self.page == page;
        let icon = match page {
            Page::Camera => "相",
            Page::Mic => "麦",
            Page::Watermark => "印",
            Page::Profiles => "档",
        };
        let fill = if selected {
            egui::Color32::from_rgb(222, 236, 255)
        } else {
            egui::Color32::TRANSPARENT
        };
        let response = egui::Frame::new()
            .fill(fill)
            .corner_radius(8.0)
            .inner_margin(egui::Margin::symmetric(10, 7))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(icon)
                            .strong()
                            .color(egui::Color32::from_rgb(0, 95, 184)),
                    );
                    ui.label(egui::RichText::new(title).strong());
                    ui.label(
                        egui::RichText::new(subtitle)
                            .small()
                            .color(egui::Color32::from_rgb(92, 99, 112)),
                    );
                });
            })
            .response;
        if response.interact(egui::Sense::click()).clicked() {
            self.page = page;
        }
        ui.add_space(2.0);
    }

    fn card(ui: &mut egui::Ui, title: &str, subtitle: &str, add: impl FnOnce(&mut egui::Ui)) {
        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(255, 255, 255))
            .corner_radius(8.0)
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                ui.label(egui::RichText::new(title).heading().strong());
                if !subtitle.is_empty() {
                    ui.label(
                        egui::RichText::new(subtitle).color(egui::Color32::from_rgb(92, 99, 112)),
                    );
                }
                ui.add_space(10.0);
                add(ui);
            });
        ui.add_space(12.0);
    }

    fn step_card(
        ui: &mut egui::Ui,
        step: &str,
        title: &str,
        subtitle: &str,
        add: impl FnOnce(&mut egui::Ui),
    ) {
        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(255, 255, 255))
            .corner_radius(8.0)
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(step)
                            .color(egui::Color32::WHITE)
                            .background_color(egui::Color32::from_rgb(0, 95, 184))
                            .strong(),
                    );
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(title).heading().strong());
                        ui.label(
                            egui::RichText::new(subtitle)
                                .color(egui::Color32::from_rgb(92, 99, 112)),
                        );
                    });
                });
                ui.add_space(12.0);
                add(ui);
            });
        ui.add_space(12.0);
    }

    fn log_panel(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("运行日志")
            .default_open(true)
            .show(ui, |ui| {
                for item in self.log.iter().rev().take(6) {
                    ui.label(item);
                }
            });
    }

    fn camera_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("Luna Ultra");
        ui.label("建议流程：连接相机 Wi-Fi，检测连接，读取素材，再下载文件。");
        ui.add_space(12.0);

        Self::step_card(
            ui,
            "1",
            "连接相机",
            "只检测 HTTP 服务，不触碰 6666 控制口，避免误断开。",
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label("相机地址");
                    ui.add(egui::TextEdit::singleline(&mut self.host).desired_width(180.0));
                    if ui.button("检测连接").clicked() {
                        self.detect_luna();
                    }
                });
            },
        );

        Self::step_card(
            ui,
            "2",
            "读取素材",
            "打开 UCD2 会话并读取相机内部目录。",
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button("刷新素材列表").clicked() {
                        self.list_luna_media();
                    }
                    if ui.button("断开会话").clicked() {
                        self.stop_luna_keeper();
                    }
                    ui.label("目录：/storage_internal/DCIM/Camera01/");
                });
            },
        );

        self.media_gallery(ui);

        Self::step_card(
            ui,
            "3",
            "下载文件",
            "从相册卡片填入 URL，或手动粘贴文件地址。",
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    let width = (ui.available_width() - 96.0).max(220.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut self.file_url)
                            .hint_text("粘贴文件 URL")
                            .desired_width(width),
                    );
                    if ui.button("开始下载").clicked() {
                        self.download_luna_file();
                    }
                });
            },
        );

        egui::CollapsingHeader::new("高级：OSC 备用命令")
            .default_open(false)
            .show(ui, |ui| {
                Self::card(
                    ui,
                    "OSC 调试",
                    "Luna Ultra 固件可能对 /osc/info 返回 404。",
                    |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label("端口");
                            ui.add(egui::TextEdit::singleline(&mut self.port).desired_width(72.0));
                            if ui.button("读取 /osc/info").clicked() {
                                self.run_camera("OSC 信息", |c| c.info());
                            }
                            if ui.button("读取 /osc/state").clicked() {
                                self.run_camera("OSC 状态", |c| c.state());
                            }
                        });
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("照片模式").clicked() {
                                self.run_camera("照片模式", |c| {
                                    c.set_options(json!({"captureMode":"image"}))
                                });
                            }
                            if ui.button("拍照").clicked() {
                                self.run_camera("拍照", |c| {
                                    c.execute("camera.takePicture", None)
                                });
                            }
                            if ui.button("视频模式").clicked() {
                                self.run_camera("视频模式", |c| {
                                    c.set_options(json!({"captureMode":"video"}))
                                });
                            }
                            if ui.button("开始录制").clicked() {
                                self.run_camera("开始录制", |c| {
                                    c.execute("camera.startCapture", None)
                                });
                            }
                            if ui.button("停止录制").clicked() {
                                self.run_camera("停止录制", |c| {
                                    c.execute("camera.stopCapture", None)
                                });
                            }
                        });
                        ui.separator();
                        ui.label("原始命令");
                        ui.text_edit_singleline(&mut self.command_name);
                        ui.label("参数 JSON");
                        ui.text_edit_multiline(&mut self.command_params);
                        if ui.button("发送命令").clicked() {
                            self.raw_osc();
                        }
                    },
                );
            });

        Self::card(
            ui,
            "输出",
            "素材列表和操作结果会显示在这里。",
            |ui| {
                ui.add_sized(
                    [ui.available_width(), 260.0],
                    egui::TextEdit::multiline(&mut self.camera_output)
                        .font(egui::TextStyle::Monospace),
                );
            },
        );
    }

    fn media_gallery(&mut self, ui: &mut egui::Ui) {
        let count = self.media_files.len();
        let subtitle = if count == 0 {
            "点击“刷新素材列表”后，相机素材会显示在这里。".to_string()
        } else {
            format!("已读取 {count} 个素材，可直接填入下载地址。")
        };

        Self::card(ui, "相册列表", &subtitle, |ui| {
            if self.media_files.is_empty() {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("暂无素材。若相机已连接，请点击上方“刷新素材列表”。")
                        .color(egui::Color32::from_rgb(92, 99, 112)),
                );
                return;
            }

            let files = self.media_files.clone();
            let available = ui.available_width().max(1.0);
            let columns = (available / 285.0).floor().max(1.0) as usize;
            let gap = 10.0;
            let card_width = ((available - gap * (columns.saturating_sub(1) as f32))
                / columns as f32)
                .clamp(160.0, available);

            for row in files.chunks(columns) {
                ui.horizontal(|ui| {
                    for (index, file) in row.iter().enumerate() {
                        egui::Frame::new()
                            .fill(egui::Color32::from_rgb(247, 250, 255))
                            .stroke(egui::Stroke::new(
                                1.0,
                                egui::Color32::from_rgb(220, 229, 242),
                            ))
                            .corner_radius(8.0)
                            .inner_margin(egui::Margin::same(12))
                            .show(ui, |ui| {
                                ui.set_width(card_width - 24.0);
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&file.name)
                                            .strong()
                                            .color(egui::Color32::from_rgb(27, 32, 43)),
                                    )
                                    .wrap(),
                                );
                                ui.add_space(6.0);
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{}  ·  {}  ·  {} {}",
                                        file.kind, file.size_text, file.date, file.time
                                    ))
                                    .small()
                                    .color(egui::Color32::from_rgb(92, 99, 112)),
                                );
                                ui.add_space(10.0);
                                ui.horizontal_wrapped(|ui| {
                                    if ui.button("填入下载").clicked() {
                                        self.file_url = file.url.clone();
                                    }
                                    if ui.button("复制 URL").clicked() {
                                        ui.ctx().copy_text(file.url.clone());
                                        self.log.push("已复制素材 URL".to_string());
                                    }
                                });
                            });
                        if index + 1 < row.len() {
                            ui.add_space(gap);
                        }
                    }
                });
                ui.add_space(10.0);
            }
        });
    }

    fn mic_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("Mic Pro");
        ui.label("扫描 Mic Pro RX/TX，查看 GATT 服务。专用控制需要后续确认 UUID 和命令。");
        ui.add_space(12.0);
        Self::card(
            ui,
            "蓝牙连接",
            "扫描附近 BLE 设备，选择 Mic Pro 后读取服务。",
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button("扫描设备").clicked() {
                        self.scan_ble();
                    }
                    ui.label("设备地址");
                    let width = ui.available_width().clamp(180.0, 320.0);
                    ui.add(egui::TextEdit::singleline(&mut self.ble_address).desired_width(width));
                    if ui.button("读取服务").clicked() {
                        self.inspect_ble();
                    }
                });
            },
        );
        Self::card(
            ui,
            "原始 GATT 写入",
            "用于后续调试 Mic Pro 命令。",
            |ui| {
                ui.label("Characteristic UUID");
                ui.text_edit_singleline(&mut self.gatt_uuid);
                ui.label("Hex 数据");
                ui.horizontal_wrapped(|ui| {
                    ui.text_edit_singleline(&mut self.gatt_hex);
                    if ui.button("写入").clicked() {
                        self.write_ble();
                    }
                });
            },
        );
        Self::card(ui, "蓝牙输出", "", |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut self.ble_output)
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(20),
            );
        });
    }

    fn watermark_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("水印导出");
        ui.label("使用从 APK 提取的 Luna Ultra 水印素材和固定参数。");
        ui.add_space(12.0);
        Self::card(ui, "媒体文件", "选择输入和输出路径。", |ui| {
            ui.label("输入文件");
            ui.text_edit_singleline(&mut self.input_media);
            ui.label("输出文件");
            ui.text_edit_singleline(&mut self.output_media);
        });
        Self::card(
            ui,
            "水印样式",
            "照片使用 App 固定位置；视频可选择 App 支持的位置。",
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label("位置");
                    egui::ComboBox::from_id_salt("position")
                        .selected_text(position_label(&self.wm_position))
                        .show_ui(ui, |ui| {
                            for (value, label) in [
                                ("bottom-left", "左下角"),
                                ("bottom-center", "底部居中"),
                                ("bottom-right", "右下角"),
                                ("top-right", "右上角"),
                                ("top-left", "左上角"),
                            ] {
                                ui.selectable_value(
                                    &mut self.wm_position,
                                    value.to_string(),
                                    label,
                                );
                            }
                        });
                });
                if ui.button("导出水印文件").clicked() {
                    self.export_watermark();
                }
            },
        );
    }

    fn profiles_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("设备档案");
        ui.label("从 APK 提取的 z03 Luna Ultra、rxTRC Mic Pro RX、txTRC Mic Pro TX 配置。");
        ui.add_space(12.0);
        Self::card(ui, "配置摘要", "", |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut self.profiles_text)
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(30),
            );
        });
    }

    fn client_from_ui(host: String, port: String) -> anyhow::Result<OscClient> {
        OscClient::new(&host, port.parse().unwrap_or(80))
    }

    fn run_camera<F>(&mut self, label: &'static str, f: F)
    where
        F: FnOnce(OscClient) -> anyhow::Result<Value> + Send + 'static,
    {
        let host = self.host.clone();
        let port = self.port.clone();
        self.log.push(format!("开始：{label}"));
        self.jobs.spawn(move || {
            let client = Self::client_from_ui(host, port)?;
            let value = f(client)?;
            Ok(serde_json::to_string_pretty(&value)?)
        });
    }

    fn raw_osc(&mut self) {
        let name = self.command_name.clone();
        let raw = self.command_params.trim().to_string();
        self.run_camera("原始 OSC", move |c| {
            let params = if raw.is_empty() {
                None
            } else {
                Some(serde_json::from_str(&raw)?)
            };
            c.execute(&name, params)
        });
    }

    fn detect_luna(&mut self) {
        let host = self.host.clone();
        self.log.push("开始：检测 Luna".to_string());
        self.jobs.spawn(move || {
            let status = adapters::luna_local::check_status(&host, false);
            Ok(serde_json::to_string_pretty(&status)?)
        });
    }

    fn list_luna_media(&mut self) {
        self.stop_luna_keeper();
        let host = self.host.clone();
        let tx = self.jobs.sender();
        let (stop_tx, stop_rx) = mpsc::channel();
        self.luna_stop = Some(stop_tx);
        self.luna_session_active = true;
        self.log
            .push("开始：打开 Luna 会话并刷新素材列表".to_string());

        std::thread::spawn(move || {
            let result = (|| -> anyhow::Result<()> {
                let mut session = adapters::luna_local::LunaAuthSession::open(&host)?;
                let files = adapters::luna_local::list_files_with_session(&host, &mut session)?;
                let _ = tx.send(Ok(serde_json::to_string_pretty(&files)?));

                loop {
                    if stop_rx.recv_timeout(Duration::from_secs(5)).is_ok() {
                        session.close();
                        let _ = tx.send(Ok("Luna 会话已断开".to_string()));
                        break;
                    }

                    if let Err(err) = session.refresh().and_then(|_| {
                        adapters::luna_local::list_files_with_session(&host, &mut session)
                            .map(|_| ())
                    }) {
                        let _ = tx.send(Err(err));
                        break;
                    }
                }

                Ok(())
            })();

            if let Err(err) = result {
                let _ = tx.send(Err(err));
            }
        });
    }

    fn stop_luna_keeper(&mut self) {
        if let Some(stop) = self.luna_stop.take() {
            let _ = stop.send(());
        }
        self.luna_session_active = false;
    }

    fn download_luna_file(&mut self) {
        let host = self.host.clone();
        let url = self.file_url.trim().to_string();
        self.log.push("开始：下载 Luna 文件".to_string());
        self.jobs.spawn(move || {
            if url.is_empty() {
                anyhow::bail!("文件 URL 不能为空");
            }
            let filename = url
                .rsplit('/')
                .next()
                .filter(|s| !s.is_empty())
                .unwrap_or("camera_file");
            let output = PathBuf::from("downloads").join(filename);
            adapters::luna_local::resume_download(&host, &url, &output)?;
            Ok(format!("已下载到 {}", output.display()))
        });
    }

    fn scan_ble(&mut self) {
        self.log.push("开始：扫描蓝牙".to_string());
        self.jobs.spawn(move || {
            let runtime = tokio::runtime::Runtime::new()?;
            let devices = runtime.block_on(adapters::mic_ble::scan_mic_devices())?;
            Ok(format!(
                "[BLE]\n{}",
                serde_json::to_string_pretty(&devices)?
            ))
        });
    }

    fn inspect_ble(&mut self) {
        let address = self.ble_address.trim().to_string();
        self.log.push("开始：读取蓝牙服务".to_string());
        self.jobs.spawn(move || {
            let runtime = tokio::runtime::Runtime::new()?;
            let chars = runtime.block_on(adapters::mic_ble::inspect(&address))?;
            Ok(format!("[BLE]\n{}", serde_json::to_string_pretty(&chars)?))
        });
    }

    fn write_ble(&mut self) {
        let address = self.ble_address.trim().to_string();
        let uuid = self.gatt_uuid.trim().to_string();
        let hex = self.gatt_hex.trim().to_string();
        self.log.push("开始：写入 GATT".to_string());
        self.jobs.spawn(move || {
            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(adapters::mic_ble::write_hex(&address, &uuid, &hex))?;
            Ok("[BLE]\n写入完成".to_string())
        });
    }

    fn export_watermark(&mut self) {
        let options = WatermarkOptions {
            input: self.input_media.trim().into(),
            output: self.output_media.trim().into(),
            position: self.wm_position.clone(),
            style: "luna-ultra-cn".to_string(),
            frame_background: "black".to_string(),
            moment_preset: "official".to_string(),
            moment_image: None,
        };
        self.log.push("开始：导出水印".to_string());
        self.jobs.spawn(move || {
            adapters::watermark::apply(&options)?;
            Ok(format!("水印文件已导出到 {}", options.output.display()))
        });
    }
}

fn position_label(value: &str) -> &'static str {
    match value {
        "bottom-left" => "左下角",
        "bottom-center" => "底部居中",
        "bottom-right" => "右下角",
        "top-right" => "右上角",
        "top-left" => "左上角",
        _ => "底部居中",
    }
}

fn configure_fonts(ctx: &egui::Context) {
    let font_paths = [
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\simhei.ttf",
        r"C:\Windows\Fonts\simsun.ttc",
    ];

    let Some(bytes) = font_paths.iter().find_map(|path| std::fs::read(path).ok()) else {
        return;
    };

    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "windows_cjk".to_string(),
        egui::FontData::from_owned(bytes).into(),
    );

    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, "windows_cjk".to_string());
    }

    ctx.set_fonts(fonts);
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Luna 控制台")
            .with_inner_size([1180.0, 780.0])
            .with_min_inner_size([980.0, 680.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Luna 控制台",
        options,
        Box::new(|cc| {
            configure_fonts(&cc.egui_ctx);
            Ok(Box::<AppState>::default())
        }),
    )
}
