use crate::auth::{poll_auth_status, request_device_code, AuthStatus, DeviceCodeInfo};
use crate::config::AppConfig;
use crate::theme;
use eframe::egui;
use std::sync::{Arc, Mutex};

enum SetupStep {
    Welcome,
    LoginPoll,
    DeviceCode,
    Directory,
    Autostart,
    Done,
}

pub struct SetupWizard {
    step: SetupStep,
    server_url: String,
    server_url_edit: String,
    code_info: Option<DeviceCodeInfo>,
    auth_status: Arc<Mutex<String>>,
    auth_error: Option<String>,
    poll_started: bool,
    rnotes_dir_edit: String,
    auto_start: bool,
    sync_subdirs: bool,
    logo: Option<egui::TextureHandle>,
}

impl SetupWizard {
    pub fn new() -> Self {
        let default_dir = dirs::document_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("Rnotes")
            .to_string_lossy()
            .to_string();

        Self {
            step: SetupStep::Welcome,
            server_url: "https://notes.huebler.tech".to_string(),
            server_url_edit: "https://notes.huebler.tech".to_string(),
            code_info: None,
            auth_status: Arc::new(Mutex::new(String::new())),
            auth_error: None,
            poll_started: false,
            rnotes_dir_edit: default_dir,
            auto_start: true,
            sync_subdirs: true,
            logo: None,
        }
    }

    fn load_logo(&mut self, ui: &egui::Ui) -> Option<&egui::TextureHandle> {
        if self.logo.is_none() {
            let img = image::load_from_memory(include_bytes!("../../logo.png")).ok()?;
            let rgba = img.to_rgba8();
            let (w, h) = rgba.dimensions();
            let color_image = egui::ColorImage::from_rgba_unmultiplied([w as _, h as _], rgba.as_raw());
            self.logo = Some(ui.ctx().load_texture("setup_logo", color_image, Default::default()));
        }
        self.logo.as_ref()
    }

    fn open_browser(&self, url: &str) {
        #[cfg(target_os = "linux")]
        std::process::Command::new("xdg-open").arg(url).spawn().ok();
        #[cfg(target_os = "windows")]
        std::process::Command::new("cmd").args(["/c", "start", url]).spawn().ok();
        #[cfg(target_os = "macos")]
        std::process::Command::new("open").arg(url).spawn().ok();
    }

    fn start_polling(&mut self, info: DeviceCodeInfo) {
        if self.poll_started { return; }
        self.poll_started = true;
        let status_clone = self.auth_status.clone();
        let server = self.server_url.clone();
        let dc = info.device_code.clone();
        let interval = info.interval;
        let expires = info.expires_in;

        std::thread::spawn(move || {
            poll_auth_status(&server, &dc, interval, expires, &|s| {
                let msg = match s {
                    AuthStatus::Authorized { access_token } => access_token,
                    AuthStatus::Pending => String::new(),
                    AuthStatus::Expired => "EXPIRED".to_string(),
                    AuthStatus::Error(e) => e,
                };
                let mut st = status_clone.lock().unwrap();
                *st = msg;
            });
        });
    }
}

impl eframe::App for SetupWizard {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let style = theme::dark_theme();
        ui.ctx().set_style_of(egui::Theme::Dark, style);

        egui::CentralPanel::default().show(ui, |ui| {
            ui.visuals_mut().window_fill = egui::Color32::from_rgb(0x1e, 0x1e, 0x1e);

            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.1);

                if let Some(logo) = self.load_logo(ui) {
                    ui.add(egui::Image::new(logo).max_width(96.0));
                    ui.add_space(8.0);
                }

                match self.step {
                    SetupStep::Welcome => {
                        ui.heading(egui::RichText::new("Welcome to SyncNotes").size(32.0).strong());
                        ui.add_space(12.0);
                        ui.label("Sync your Rnote files to your private server.");
                        ui.add_space(32.0);

                        ui.label("Server URL:");
                        ui.add(egui::TextEdit::singleline(&mut self.server_url_edit)
                            .hint_text("https://notes.huebler.tech")
                            .desired_width(320.0)
                            .margin(egui::Margin::symmetric(8, 8)));
                        self.server_url = self.server_url_edit.trim().to_string();

                        ui.add_space(32.0);

                        if ui.add(egui::Button::new(egui::RichText::new("Login with Browser").size(18.0))
                            .min_size(egui::vec2(280.0, 48.0)))
                            .clicked() 
                        {
                            match request_device_code(&self.server_url) {
                                Ok(info) => {
                                    self.open_browser(&info.verification_uri);
                                    self.code_info = Some(info.clone());
                                    self.start_polling(info);
                                    self.step = SetupStep::LoginPoll;
                                }
                                Err(e) => { self.auth_error = Some(e); }
                            }
                        }

                        ui.add_space(16.0);
                        if ui.add(egui::Button::new("Login with Device Code").min_size(egui::vec2(200.0, 30.0))).clicked() {
                            self.step = SetupStep::DeviceCode;
                        }
                    }

                    SetupStep::LoginPoll => {
                        ui.heading("Authorizing...");
                        ui.add_space(16.0);
                        ui.label("Please complete the authorization in your browser.");
                        ui.add_space(24.0);
                        ui.add(egui::ProgressBar::new(0.5).animate(true).desired_width(240.0));
                        ui.add_space(24.0);
                        
                        let status = self.auth_status.lock().unwrap().clone();
                        if status == "EXPIRED" || status.starts_with("Connection") || status.starts_with("HTTP") {
                            ui.colored_label(egui::Color32::from_rgb(0xe3, 0x3f, 0x3f), &status);
                            if ui.button("Retry").clicked() {
                                self.step = SetupStep::Welcome;
                                self.poll_started = false;
                                *self.auth_status.lock().unwrap() = String::new();
                            }
                        } else if !status.is_empty() {
                            let mut config = AppConfig::load().unwrap_or_default();
                            config.access_token = status;
                            config.server_url = self.server_url.clone();
                            config.save();
                            self.step = SetupStep::Directory;
                        }

                        if ui.button("Cancel").clicked() {
                            self.step = SetupStep::Welcome;
                            self.poll_started = false;
                            *self.auth_status.lock().unwrap() = String::new();
                        }
                    }

                    SetupStep::DeviceCode => {
                        ui.heading("Device Code Login");
                        ui.add_space(16.0);

                        if self.code_info.is_none() && self.auth_error.is_none() {
                            match request_device_code(&self.server_url) {
                                Ok(info) => { self.code_info = Some(info); }
                                Err(e) => { self.auth_error = Some(e); }
                            }
                        }

                        if let Some(ref info) = self.code_info {
                            ui.label("Enter this code on the authorization page:");
                            ui.add_space(16.0);

                            egui::Frame {
                                fill: egui::Color32::from_rgb(0x2a, 0x2a, 0x2a),
                                corner_radius: egui::CornerRadius::same(8),
                                inner_margin: egui::Margin::symmetric(24, 24),
                                ..Default::default()
                            }.show(ui, |ui| {
                                    ui.heading(egui::RichText::new(&info.user_code)
                                        .size(36.0)
                                        .color(egui::Color32::from_rgb(0x35, 0x84, 0xe4))
                                        .monospace());
                                });

                            ui.add_space(24.0);
                            if ui.button("Open Authorization Page").clicked() {
                                self.open_browser(&info.verification_uri);
                            }
                            self.start_polling(info.clone());
                        }

                        ui.add_space(24.0);
                        let status = self.auth_status.lock().unwrap().clone();
                        if !status.is_empty() && status != "EXPIRED" && !status.contains("Error") {
                             let mut config = AppConfig::load().unwrap_or_default();
                             config.access_token = status;
                             config.server_url = self.server_url.clone();
                             config.save();
                             self.step = SetupStep::Directory;
                        }

                        if ui.button("Back").clicked() {
                            self.step = SetupStep::Welcome;
                        }
                    }

                    SetupStep::Directory => {
                        ui.heading("Notes Directory");
                        ui.add_space(16.0);
                        ui.label("Select the folder containing your .rnote files:");
                        ui.add_space(16.0);

                        ui.horizontal(|ui| {
                            ui.add_space((ui.available_width() - 360.0) / 2.0);
                            ui.add(egui::TextEdit::singleline(&mut self.rnotes_dir_edit).desired_width(280.0));
                            if ui.button("Browse...").clicked() {
                                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                    self.rnotes_dir_edit = path.to_string_lossy().to_string();
                                }
                            }
                        });

                        ui.add_space(32.0);
                        if ui.add(egui::Button::new("Continue").min_size(egui::vec2(200.0, 40.0))).clicked() {
                            self.step = SetupStep::Autostart;
                        }
                    }

                    SetupStep::Autostart => {
                        ui.heading("Settings");
                        ui.add_space(24.0);
                        
                        ui.checkbox(&mut self.auto_start, "Start automatically on login");
                        ui.add_space(8.0);
                        ui.checkbox(&mut self.sync_subdirs, "Sync subdirectories");
                        
                        ui.add_space(48.0);
                        if ui.add(egui::Button::new("Finish Setup").min_size(egui::vec2(200.0, 48.0))).clicked() {
                            let mut config = AppConfig::load().unwrap_or_default();
                            config.rnotes_dir = self.rnotes_dir_edit.trim().to_string();
                            config.autostart = self.auto_start;
                            config.sync_subdirs = self.sync_subdirs;
                            config.save();

                            if self.auto_start {
                                if let Ok(launcher) = auto_launch::AutoLaunchBuilder::new()
                                    .set_app_name("SyncNotes")
                                    .set_app_path(std::env::current_exe().unwrap_or_default().to_string_lossy().as_ref())
                                    .set_args(&["--daemon"])
                                    .build() {
                                    launcher.enable().ok();
                                }
                            }
                            self.step = SetupStep::Done;
                        }
                    }

                    SetupStep::Done => {
                        ui.heading("✨ Setup Complete! ✨");
                        ui.add_space(24.0);
                        ui.label("SyncNotes is running in the background and will keep your files in sync.");
                        ui.add_space(48.0);
                        if ui.add(egui::Button::new("Close").min_size(egui::vec2(160.0, 40.0))).clicked() {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    }
                }
                
                if let Some(ref err) = self.auth_error {
                    ui.add_space(16.0);
                    ui.colored_label(egui::Color32::from_rgb(0xe3, 0x3f, 0x3f), err);
                    if ui.button("Clear Error").clicked() { self.auth_error = None; }
                }

                ui.add_space(ui.available_height().max(0.0));
            });
        });
        
        ui.ctx().request_repaint();
    }
}
