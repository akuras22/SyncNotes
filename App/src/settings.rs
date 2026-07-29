use crate::config::AppConfig;
use crate::theme;
use crate::tray;
use eframe::egui;
use std::sync::atomic::{AtomicBool, Ordering};

pub static DISCONNECT_REQUESTED: AtomicBool = AtomicBool::new(false);

pub struct SettingsWindow {
    config: AppConfig,
    server_url_edit: String,
    rnotes_dir_edit: String,
    message: String,
    logo: Option<egui::TextureHandle>,
}

impl SettingsWindow {
    pub fn new(config: AppConfig) -> Self {
        Self {
            server_url_edit: config.server_url.clone(),
            rnotes_dir_edit: config.rnotes_dir.clone(),
            message: String::new(),
            config,
            logo: None,
        }
    }

    fn load_logo(&mut self, ui: &egui::Ui) -> Option<&egui::TextureHandle> {
        if self.logo.is_none() {
            let img = image::load_from_memory(include_bytes!("../../syncnotes-icon.png")).ok()?;
            let rgba = img.to_rgba8();
            let (w, h) = rgba.dimensions();
            let color_image = egui::ColorImage::from_rgba_unmultiplied([w as _, h as _], rgba.as_raw());
            self.logo = Some(ui.ctx().load_texture("settings_logo", color_image, Default::default()));
        }
        self.logo.as_ref()
    }
}

impl eframe::App for SettingsWindow {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx();

        if tray::SHOULD_QUIT.load(Ordering::Relaxed) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        if let Some(cmd) = egui::ViewportCommand::center_on_screen(ctx) {
            ctx.send_viewport_cmd(cmd);
        }
        let style = theme::dark_theme();
        ui.ctx().set_style_of(egui::Theme::Dark, style);

        egui::CentralPanel::default().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.08);

                if let Some(logo) = self.load_logo(ui) {
                    ui.add(egui::Image::new(logo).max_width(80.0));
                    ui.add_space(8.0);
                }
                ui.heading(egui::RichText::new("SyncNotes Settings").size(24.0).strong());
                ui.add_space(24.0);

                egui::Frame {
                    fill: egui::Color32::from_rgb(0x1b, 0x1b, 0x22),
                    corner_radius: egui::CornerRadius::same(14),
                    stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(0x35, 0x35, 0x40)),
                    inner_margin: egui::Margin::symmetric(18, 18),
                    ..Default::default()
                }.show(ui, |ui| {
                    ui.set_max_width(440.0);
                    egui::Grid::new("settings_grid")
                        .spacing([12.0, 12.0])
                        .min_col_width(120.0)
                        .show(ui, |ui| {
                            ui.label("Server URL:");
                            ui.add(egui::TextEdit::singleline(&mut self.server_url_edit).desired_width(280.0));
                            ui.end_row();

                            ui.label("Rnotes Directory:");
                            ui.horizontal(|ui| {
                                ui.add(egui::TextEdit::singleline(&mut self.rnotes_dir_edit).desired_width(210.0));
                                if ui.button("Browse").clicked() {
                                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                        self.rnotes_dir_edit = path.to_string_lossy().to_string();
                                    }
                                }
                            });
                            ui.end_row();

                            ui.label("Auto-start:");
                            if ui.checkbox(&mut self.config.autostart, "Start on login").changed() {
                                let launcher = auto_launch::AutoLaunchBuilder::new()
                                    .set_app_name("SyncNotes")
                                    .set_app_path(std::env::current_exe().unwrap_or_default().to_string_lossy().as_ref())
                                    .set_args(&["--daemon"])
                                    .build();
                                if let Ok(l) = launcher {
                                    if self.config.autostart { l.enable().ok(); } else { l.disable().ok(); }
                                }
                            }
                            ui.end_row();

                            ui.label("Sync:");
                            ui.checkbox(&mut self.config.sync_subdirs, "Include subdirectories");
                            ui.end_row();

                            ui.label("Tray Icon:");
                            ui.checkbox(&mut self.config.show_tray_icon, "Show in system tray");
                            ui.end_row();
                        });
                });

                ui.add_space(24.0);

                ui.horizontal(|ui| {
                    ui.add_space((ui.available_width() - 260.0) / 2.0);
                    if theme::primary_button(ui, "Save", egui::vec2(120.0, 36.0)).clicked() {
                        self.config.server_url = self.server_url_edit.trim().to_string();
                        self.config.rnotes_dir = self.rnotes_dir_edit.trim().to_string();
                        self.config.save();
                        self.message = "Settings saved successfully.".to_string();
                    }

                    ui.add_space(4.0);

                    if ui.add(egui::Button::new(
                        egui::RichText::new("Disconnect").color(theme::DESTRUCTIVE),
                    ).min_size(egui::vec2(120.0, 36.0))).clicked() {
                        // Best-effort: also revoke this device's token server-side so it
                        // disappears from the website's Authorized Apps list. If the
                        // server is unreachable we still disconnect locally either way.
                        crate::auth::revoke_token(&self.config.server_url, &self.config.access_token);
                        AppConfig::delete();
                        DISCONNECT_REQUESTED.store(true, Ordering::Relaxed);
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                if !self.message.is_empty() {
                    ui.add_space(16.0);
                    let color = if self.message.contains("Disconnected") {
                        egui::Color32::from_rgb(0xe6, 0xa2, 0x3c)
                    } else {
                        theme::SUCCESS
                    };
                    ui.colored_label(color, &self.message);
                }

            });
        });
    }
}
