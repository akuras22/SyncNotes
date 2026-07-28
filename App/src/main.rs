mod auth;
mod config;
mod settings;
mod setup;
mod theme;

use config::AppConfig;
use eframe::egui;
use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let is_settings = args.iter().any(|a| a == "--settings");
    let config = AppConfig::load();

    if config.is_none() {
        run_setup();
    } else if is_settings {
        run_settings(config.unwrap());
    } else {
        run_daemon();
    }
}

fn run_setup() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([540.0, 480.0])
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

fn run_daemon() {
    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}
