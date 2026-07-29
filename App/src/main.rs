mod auth;
mod config;
mod settings;
mod setup;
mod theme;
mod tray;

use config::AppConfig;
use eframe::egui;
use std::time::Duration;

fn main() {
    let config = AppConfig::load();

    match config {
        None => run_setup(),
        Some(cfg) => {
            if cfg.show_tray_icon {
                tray::start_tray();
                run_settings(cfg);
                loop {
                    std::thread::sleep(Duration::from_secs(86400));
                }
            } else {
                run_settings(cfg);
            }
        }
    }
}

fn run_setup() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([480.0, 560.0])
            .with_resizable(false)
            .with_title("SyncNotes Setup"),
        ..Default::default()
    };

    eframe::run_native(
        "SyncNotes Setup",
        options,
        Box::new(|_cc| Ok(Box::new(setup::SetupWizard::new()))),
    )
    .ok();
}

fn run_settings(config: AppConfig) {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([540.0, 400.0])
            .with_resizable(false)
            .with_title("SyncNotes Settings"),
        ..Default::default()
    };

    eframe::run_native(
        "SyncNotes Settings",
        options,
        Box::new(|_cc| Ok(Box::new(settings::SettingsWindow::new(config)))),
    )
    .ok();
}
