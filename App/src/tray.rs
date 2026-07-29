use std::sync::atomic::{AtomicBool, Ordering};
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub static SHOULD_QUIT: AtomicBool = AtomicBool::new(false);

pub fn create_tray() -> Option<TrayIcon> {
    let img = image::load_from_memory(include_bytes!("../../logo.png")).ok()?;
    let small = img.resize_exact(64, 64, image::imageops::FilterType::Lanczos3);
    let rgba = small.to_rgba8();
    let icon = Icon::from_rgba(rgba.as_raw().clone(), rgba.width(), rgba.height()).ok()?;

    let show = MenuItem::new("Show Settings", true, None);
    let quit = MenuItem::new("Quit", true, None);
    let quit_id = quit.id();

    let menu = Menu::new();
    menu.append_items(&[&show, &quit]).ok()?;

    MenuEvent::set_event_handler(Some(move |event: tray_icon::menu::MenuEvent| {
        if event.id == quit_id {
            SHOULD_QUIT.store(true, Ordering::Relaxed);
        }
    }));

    let tray = TrayIconBuilder::new()
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .with_tooltip("SyncNotes")
        .build()
        .ok()?;

    Some(tray)
}
