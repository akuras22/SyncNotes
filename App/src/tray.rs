use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, TrayIconBuilder};

use crate::icon::load_logo_rgba;

pub static SHOULD_QUIT: AtomicBool = AtomicBool::new(false);

pub fn create_tray() -> Option<JoinHandle<()>> {
    let icon = make_tray_icon()?;

    let handle = thread::Builder::new()
        .name("tray".into())
        .spawn(move || {
            if gtk::init().is_err() {
                return;
            }

            let show = MenuItem::new("Show Settings", true, None);
            let quit = MenuItem::new("Quit", true, None);
            let quit_id = quit.id().clone();

            let menu = Menu::new();
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

            glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
                if SHOULD_QUIT.load(Ordering::Relaxed) {
                    gtk::main_quit();
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            });
            gtk::main();
        })
        .ok()?;

    Some(handle)
}

fn make_tray_icon() -> Option<Icon> {
    let (rgba, w, h) = load_logo_rgba(128)?;
    Icon::from_rgba(rgba, w, h).ok()
}
