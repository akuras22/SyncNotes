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
            run_settings(cfg);
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

fn run_settings(mut config: AppConfig) {
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    let tray_handle = if config.show_tray_icon {
        tray::create_tray()
    } else {
        None
    };

    let tray_alive = tray_handle.is_some();

    'outer: loop {
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

        if !tray_alive || tray::SHOULD_QUIT.load(Ordering::Relaxed) {
            break;
        }

        loop {
            std::thread::sleep(Duration::from_millis(200));
            if tray::SHOULD_SHOW.swap(false, Ordering::Relaxed) {
                config = config::AppConfig::load().unwrap_or_default();
                continue 'outer;
            }
            if tray::SHOULD_QUIT.load(Ordering::Relaxed) {
                break 'outer;
            }
        }
    }

    if let Some(handle) = tray_handle {
        handle.join().ok();
    }
}
