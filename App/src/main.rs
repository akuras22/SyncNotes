mod auth;
mod config;
mod settings;
mod setup;
mod theme;
mod tray;

use config::AppConfig;
use eframe::egui;

fn load_icon() -> Option<egui::IconData> {
    let img = image::load_from_memory(include_bytes!("../../logo.png")).ok()?;
    let small = img.resize_exact(64, 64, image::imageops::FilterType::Lanczos3);
    let rgba = small.to_rgba8();
    let (w, h) = rgba.dimensions();
    Some(egui::IconData {
        rgba: rgba.as_raw().clone(),
        width: w,
        height: h,
    })
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
        Box::new(|_cc| Ok(Box::new(settings::SettingsWindow::new(config)))),
    )
    .ok();
}
