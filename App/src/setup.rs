use crate::auth::{poll_auth_status, request_device_code, AuthStatus, DeviceCodeInfo};
use crate::config::AppConfig;
use crate::theme;
use eframe::egui;
use std::sync::{Arc, Mutex};

enum SetupStep {
    Welcome,
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
        }
    }
}

impl eframe::App for SetupWizard {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let style = theme::dark_theme();
        ui.ctx().set_style_of(egui::Theme::Dark, style);

        egui::CentralPanel::default().show(ui, |ui| {
            ui.visuals_mut().window_fill =
                egui::Color32::from_rgb(0x1e, 0x1e, 0x1e);

            ui.add_space(24.0);

            match self.step {
                SetupStep::Welcome => {
                    ui.heading("Welcome to SyncNotes");
                    ui.add_space(8.0);
                    ui.label(
                        "This app syncs your Rnote files to your SyncNotes server.",
                    );
                    ui.add_space(12.0);
                    ui.label("Server URL:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.server_url_edit)
                            .hint_text("https://notes.huebler.tech")
                            .desired_width(400.0),
                    );
                    self.server_url = self.server_url_edit.trim().to_string();
                    ui.add_space(16.0);
                    if ui
                        .add(egui::Button::new("Login with Device Code").min_size(
                            egui::vec2(220.0, 36.0),
                        ))
                        .clicked()
                    {
                        self.step = SetupStep::DeviceCode;
                    }
                }

                SetupStep::DeviceCode => {
                    ui.heading("Login");
                    ui.add_space(8.0);

                    if self.code_info.is_none() && self.auth_error.is_none() {
                        match request_device_code(&self.server_url) {
                            Ok(info) => {
                                self.code_info = Some(info);
                            }
                            Err(e) => {
                                self.auth_error = Some(e);
                            }
                        }
                    }

                    if let Some(ref info) = self.code_info {
                        ui.label("Enter this code on the authorization page:");
                        ui.add_space(4.0);

                        let frame = egui::Frame {
                            fill: egui::Color32::from_rgb(0x2a, 0x2a, 0x2a),
                            corner_radius: egui::CornerRadius::same(8),
                            ..Default::default()
                        };
                        frame.show(ui, |ui| {
                            ui.add_space(12.0);
                            ui.horizontal_centered(|ui| {
                                ui.add_space(12.0);
                                ui.heading(
                                    egui::RichText::new(&info.user_code)
                                        .size(28.0)
                                        .color(egui::Color32::from_rgb(
                                            0x35, 0x84, 0xe4,
                                        ))
                                        .monospace(),
                                );
                                ui.add_space(12.0);
                            });
                            ui.add_space(12.0);
                        });

                        ui.add_space(12.0);

                        if ui
                            .add(
                                egui::Button::new("Open Browser to Authorize")
                                    .min_size(egui::vec2(260.0, 36.0)),
                            )
                            .clicked()
                        {
                            let url = info.verification_uri.clone();
                            #[cfg(target_os = "linux")]
                            std::process::Command::new("xdg-open")
                                .arg(&url)
                                .spawn()
                                .ok();
                            #[cfg(target_os = "windows")]
                            std::process::Command::new("cmd")
                                .args(["/c", "start", &url])
                                .spawn()
                                .ok();
                            #[cfg(target_os = "macos")]
                            std::process::Command::new("open")
                                .arg(&url)
                                .spawn()
                                .ok();
                        }

                        ui.add_space(12.0);

                        if !self.poll_started {
                            self.poll_started = true;
                            let status_clone = self.auth_status.clone();
                            let server = self.server_url.clone();
                            let dc = info.device_code.clone();
                            let interval = info.interval;
                            let expires = info.expires_in;

                            std::thread::spawn(move || {
                                poll_auth_status(
                                    &server,
                                    &dc,
                                    interval,
                                    expires,
                                    &|s| {
                                        let msg = match s {
                                            AuthStatus::Authorized { access_token } => {
                                                access_token
                                            }
                                            AuthStatus::Pending => String::new(),
                                            AuthStatus::Expired => {
                                                "EXPIRED".to_string()
                                            }
                                            AuthStatus::Error(e) => e,
                                        };
                                        let mut st = status_clone.lock().unwrap();
                                        *st = msg;
                                    },
                                );
                            });
                        }

                        ui.ctx().request_repaint();

                        let status = self.auth_status.lock().unwrap().clone();
                        if status.is_empty() {
                            ui.label("Waiting for authorization...");
                            ui.add(
                                egui::ProgressBar::new(0.5)
                                    .animate(true)
                                    .desired_width(300.0),
                            );
                        } else if status == "EXPIRED" {
                            ui.colored_label(
                                egui::Color32::from_rgb(0xe3, 0x3f, 0x3f),
                                "Code expired. Click below to retry.",
                            );
                            if ui.button("Retry").clicked() {
                                self.code_info = None;
                                self.auth_error = None;
                                self.poll_started = false;
                                let mut st = self.auth_status.lock().unwrap();
                                *st = String::new();
                            }
                        } else if status.starts_with("Connection")
                            || status.starts_with("HTTP")
                        {
                            ui.colored_label(
                                egui::Color32::from_rgb(0xe3, 0x3f, 0x3f),
                                &format!("Error: {}", status),
                            );
                            if ui.button("Retry").clicked() {
                                self.code_info = None;
                                self.auth_error = None;
                                self.poll_started = false;
                                let mut st = self.auth_status.lock().unwrap();
                                *st = String::new();
                            }
                        } else if !status.is_empty() {
                            ui.colored_label(
                                egui::Color32::from_rgb(0x2e, 0xc2, 0x7e),
                                "✓ Authorized!",
                            );

                            let mut config =
                                AppConfig::load().unwrap_or(AppConfig {
                                    server_url: self.server_url.clone(),
                                    access_token: String::new(),
                                    rnotes_dir: String::new(),
                                    autostart: true,
                                    sync_subdirs: true,
                                });
                            config.access_token = status;
                            config.server_url = self.server_url.clone();
                            config.save();

                            self.step = SetupStep::Directory;
                        }
                    }

                    if let Some(ref err) = self.auth_error {
                        ui.colored_label(
                            egui::Color32::from_rgb(0xe3, 0x3f, 0x3f),
                            err,
                        );
                        if ui.button("Go Back").clicked() {
                            self.auth_error = None;
                            self.step = SetupStep::Welcome;
                        }
                    }
                }

                SetupStep::Directory => {
                    ui.heading("Rnotes Directory");
                    ui.add_space(8.0);
                    ui.label("Where are your .rnote files located?");
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.rnotes_dir_edit)
                                .desired_width(320.0),
                        );
                        if ui.button("Browse...").clicked() {
                            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                self.rnotes_dir_edit =
                                    path.to_string_lossy().to_string();
                            }
                        }
                    });

                    ui.add_space(16.0);
                    if ui
                        .add(
                            egui::Button::new("Continue")
                                .min_size(egui::vec2(200.0, 36.0)),
                        )
                        .clicked()
                    {
                        self.step = SetupStep::Autostart;
                    }
                }

                SetupStep::Autostart => {
                    ui.heading("Almost done!");
                    ui.add_space(8.0);
                    ui.checkbox(&mut self.auto_start, "Start automatically on login");
                    ui.add_space(16.0);

                    if ui
                        .add(
                            egui::Button::new("Finish Setup")
                                .min_size(egui::vec2(200.0, 36.0)),
                        )
                        .clicked()
                    {
                        let token = AppConfig::load()
                            .map(|c| c.access_token)
                            .unwrap_or_default();

                        let config = AppConfig {
                            server_url: self.server_url.clone(),
                            access_token: token,
                            rnotes_dir: self.rnotes_dir_edit.trim().to_string(),
                            autostart: self.auto_start,
                            sync_subdirs: self.sync_subdirs,
                        };
                        config.save();

                        if self.auto_start {
                            if let Ok(mut launcher) = auto_launch::AutoLaunchBuilder::new()
                                .set_app_name("SyncNotes")
                                .set_app_path(
                                    std::env::current_exe()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .as_ref(),
                                )
                                .set_args(&["--daemon"])
                                .build()
                            {
                                launcher.enable().ok();
                            }
                        }

                        self.step = SetupStep::Done;
                    }
                }

                SetupStep::Done => {
                    ui.heading("Setup complete!");
                    ui.add_space(8.0);
                    ui.label(
                        "SyncNotes is now configured and ready to sync your files.",
                    );
                    ui.add_space(16.0);
                    if ui
                        .add(
                            egui::Button::new("Close")
                                .min_size(egui::vec2(200.0, 36.0)),
                        )
                        .clicked()
                    {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            }

            ui.add_space(24.0);
        });
    }
}
