use crate::config::AppConfig;
use crate::theme;
use eframe::egui;

pub struct SettingsWindow {
    config: AppConfig,
    server_url_edit: String,
    rnotes_dir_edit: String,
    message: String,
}

impl SettingsWindow {
    pub fn new(config: AppConfig) -> Self {
        Self {
            server_url_edit: config.server_url.clone(),
            rnotes_dir_edit: config.rnotes_dir.clone(),
            message: String::new(),
            config,
        }
    }
}

impl eframe::App for SettingsWindow {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let style = theme::dark_theme();
        ui.ctx().set_style_of(egui::Theme::Dark, style);

        egui::CentralPanel::default().show(ui, |ui| {
            ui.visuals_mut().window_fill = egui::Color32::from_rgb(0x1e, 0x1e, 0x1e);

            ui.vertical_centered(|ui| {
                ui.add_space(24.0);
                ui.heading(egui::RichText::new("SyncNotes Settings").size(24.0).strong());
                ui.add_space(24.0);

                let frame = egui::Frame::none()
                    .fill(egui::Color32::from_rgb(0x2d, 0x2d, 0x2d))
                    .corner_radius(8.0)
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(0x4a, 0x4a, 0x4a)))
                    .inner_margin(16.0);

                frame.show(ui, |ui| {
                    egui::Grid::new("settings_grid")
                        .spacing([12.0, 16.0])
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
                        });
                });

                ui.add_space(24.0);

                ui.horizontal(|ui| {
                    ui.add_space((ui.available_width() - 250.0) / 2.0);
                    if ui.add(egui::Button::new("Save").min_size(egui::vec2(120.0, 32.0))).clicked() {
                        self.config.server_url = self.server_url_edit.trim().to_string();
                        self.config.rnotes_dir = self.rnotes_dir_edit.trim().to_string();
                        self.config.save();
                        self.message = "Settings saved successfully.".to_string();
                    }

                    if ui.add(egui::Button::new("Disconnect").min_size(egui::vec2(120.0, 32.0))).clicked() {
                        AppConfig::delete();
                        self.message = "Disconnected. Run setup again.".to_string();
                    }
                });

                if !self.message.is_empty() {
                    ui.add_space(16.0);
                    let color = if self.message.contains("Disconnected") {
                        egui::Color32::from_rgb(0xe6, 0x61, 0x00)
                    } else {
                        egui::Color32::from_rgb(0x2e, 0xc2, 0x7e)
                    };
                    ui.colored_label(color, &self.message);
                }
            });
        });
    }
}
