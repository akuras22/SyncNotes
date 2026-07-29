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
    let (rgba, w, h) = load_logo_rgba(512)?;
    Some(egui::IconData { rgba, width: w, height: h })
}

fn main() {
    let is_daemon = std::env::args().any(|arg| arg == "--daemon");
    if is_daemon {
        run_daemon();
        return;
    }

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
        .with_app_id("com.syncnotes.desktop")
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

    if setup::SETUP_COMPLETE.swap(false, std::sync::atomic::Ordering::Relaxed) {
        if let Some(cfg) = AppConfig::load() {
            run_settings(cfg);
        }
    }
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
            .with_app_id("com.syncnotes.desktop")
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

        if settings::DISCONNECT_REQUESTED.swap(false, Ordering::Relaxed) {
            if let Some(handle) = tray_handle {
                tray::SHOULD_STOP_TRAY.store(true, Ordering::Relaxed);
                handle.join().ok();
            }
            run_setup();
            return;
        }

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

fn run_daemon() {
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    let mut tray_handle = None;

    loop {
        let config = AppConfig::load().unwrap_or_default();

        if config.show_tray_icon {
            if tray_handle.is_none() {
                tray_handle = tray::create_tray();
            }
        } else if let Some(handle) = tray_handle.take() {
            tray::SHOULD_STOP_TRAY.store(true, Ordering::Relaxed);
            handle.join().ok();
        }

        if tray::SHOULD_QUIT.load(Ordering::Relaxed) {
            break;
        }

        std::thread::sleep(Duration::from_millis(500));
    }

    if let Some(handle) = tray_handle {
        tray::SHOULD_STOP_TRAY.store(true, Ordering::Relaxed);
        handle.join().ok();
    }
}
