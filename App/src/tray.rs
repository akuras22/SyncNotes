use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, TrayIconBuilder};

pub static SHOULD_QUIT: AtomicBool = AtomicBool::new(false);

pub fn create_tray() {
    let icon = match load_tray_icon() {
        Some(i) => i,
        None => return,
    };

    thread::Builder::new()
        .name("tray".into())
        .spawn(move || {
            if gtk::init().is_err() {
                return;
            }

            let quit = MenuItem::new("Quit", true, None);
            let quit_id = quit.id().clone();

            let menu = Menu::new();
            let show = MenuItem::new("Show Settings", true, None);
            if menu.append_items(&[&show, &quit]).is_err() {
                return;
            }

            MenuEvent::set_event_handler(Some(move |event: tray_icon::menu::MenuEvent| {
                if event.id == quit_id {
                    SHOULD_QUIT.store(true, Ordering::Relaxed);
                }
            }));

            if TrayIconBuilder::new()
                .with_icon(icon)
                .with_menu(Box::new(menu))
                .with_tooltip("SyncNotes")
                .build()
                .is_err()
            {
                return;
            }

            loop {
                gtk::main_iteration_do(false);
                thread::sleep(Duration::from_millis(100));
            }
        })
        .ok();
}

fn load_tray_icon() -> Option<Icon> {
    let img = image::load_from_memory(include_bytes!("../../logo.png")).ok()?;
    let small = img.resize_exact(64, 64, image::imageops::FilterType::Lanczos3);
    let rgba = small.to_rgba8();
    Icon::from_rgba(rgba.as_raw().clone(), rgba.width(), rgba.height()).ok()
}
