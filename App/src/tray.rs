use std::sync::atomic::{AtomicBool, Ordering};

pub static TRAY_RUNNING: AtomicBool = AtomicBool::new(false);

pub fn start_tray() {
    if TRAY_RUNNING.load(Ordering::Relaxed) {
        return;
    }

    let img_bytes = include_bytes!("../../logo.png");
    let img = match image::load_from_memory(img_bytes) {
        Ok(i) => i.resize_exact(32, 32, image::imageops::FilterType::Lanczos3),
        Err(_) => return,
    };
    let rgba_img = img.to_rgba8();
    let (w, h) = rgba_img.dimensions();
    let rgba: Vec<u8> = rgba_img.as_raw().clone();
    let icon = match tray_icon::icon::Icon::from_rgba(rgba, w, h) {
        Ok(i) => i,
        Err(_) => return,
    };

    use tray_icon::menu::{Menu, MenuItem};
    let menu = Menu::new();
    let show = MenuItem::new("Open Settings", true, None);
    let quit = MenuItem::new("Quit", true, None);
    menu.append(&show);
    menu.append(&quit);

    let tray = match tray_icon::TrayIconBuilder::new()
        .with_tooltip("SyncNotes")
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .build()
    {
        Ok(t) => t,
        Err(_) => return,
    };

    TRAY_RUNNING.store(true, Ordering::Relaxed);
    Box::leak(Box::new(tray));

    let show_id = show.id();
    let quit_id = quit.id();

    std::thread::spawn(move || {
        let event_rx = tray_icon::TrayEvent::receiver();
        while let Ok(_event) = event_rx.recv() {
            let _ = std::process::Command::new(
                std::env::current_exe().unwrap_or_default(),
            )
            .spawn();
        }
    });

    std::thread::spawn(move || {
        let menu_rx = tray_icon::menu::MenuEvent::receiver();
        while let Ok(event) = menu_rx.recv() {
            if event.id == quit_id {
                std::process::exit(0);
            }
            if event.id == show_id {
                let _ = std::process::Command::new(
                    std::env::current_exe().unwrap_or_default(),
                )
                .spawn();
            }
        }
    });
}
