mod auth;
mod config;
mod icon;
mod settings;
mod setup;
mod theme;
mod tray;

use config::AppConfig;
use eframe::egui;
use icon::load_logo_rgba;

fn load_icon() -> Option<egui::IconData> {
    let (rgba, w, h) = load_logo_rgba(256)?;
    Some(egui::IconData { rgba, width: w, height: h })
}

fn main() {
    let config = AppConfig::load();

    match config {
        None => run_setup(),
        Some(cfg) => {
            let token_valid = !cfg.access_token.is_empty()
                && auth::verify_token(&cfg.server_url, &cfg.access_token);

            if token_valid {
                run_settings(cfg);
            } else {
                AppConfig::delete();
                run_setup();
            }
        }
    }
}

fn run_setup() {
    let icon = load_icon();
    let mut vp = egui::ViewportBuilder::default()
        .with_inner_size([480.0, 640.0])
        .with_resizable(false)
        .with_title("SyncNotes Setup");
    if let Some(icon) = icon {
        vp = vp.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport: vp,
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
    if config.show_tray_icon {
        tray::create_tray();
    }

    let icon = load_icon();
    let mut vp = egui::ViewportBuilder::default()
        .with_inner_size([540.0, 520.0])
        .with_resizable(false)
        .with_title("SyncNotes Settings");
    if let Some(icon) = icon {
        vp = vp.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport: vp,
        ..Default::default()
    };

    eframe::run_native(
        "SyncNotes Settings",
        options,
        Box::new(move |_cc| Ok(Box::new(settings::SettingsWindow::new(config)))),
    )
    .ok();
}
