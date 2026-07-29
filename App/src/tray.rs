use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, TrayIconBuilder, TrayIconEvent};

pub static SHOULD_QUIT: AtomicBool = AtomicBool::new(false);
pub static SHOULD_SHOW: AtomicBool = AtomicBool::new(false);

pub fn create_tray() -> Option<JoinHandle<()>> {
    let handle = thread::Builder::new()
        .name("tray".into())
        .spawn(move || {
            if gtk::init().is_err() {
                return;
            }

            let icon = load_icon();
            if icon.is_none() {
                return;
            }

            let show = MenuItem::new("Show Settings", true, None);
            let show_id = show.id().clone();
            let quit = MenuItem::new("Quit", true, None);
            let quit_id = quit.id().clone();

            let menu = Menu::new();
            if menu.append_items(&[&show, &quit]).is_err() {
                return;
            }

            TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
                if let TrayIconEvent::Click { button: tray_icon::ClickType::Left, .. } = event {
                    SHOULD_SHOW.store(true, Ordering::Relaxed);
                }
            }));

            MenuEvent::set_event_handler(Some(move |event: tray_icon::menu::MenuEvent| {
                if event.id == quit_id {
                    SHOULD_QUIT.store(true, Ordering::Relaxed);
                } else if event.id == show_id {
                    SHOULD_SHOW.store(true, Ordering::Relaxed);
                }
            }));

            if TrayIconBuilder::new()
                .with_icon(icon.unwrap())
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

fn load_icon() -> Option<Icon> {
    let img = image::load_from_memory(include_bytes!("../../logo.png")).ok()?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let dim = w.min(h);
    let x = (w - dim) / 2;
    let y = (h - dim) / 2;
    let cropped = rgba.view(x, y, dim, dim).to_image();
    let resized = image::imageops::resize(&cropped, 64, 64, image::imageops::FilterType::Lanczos3);
    Icon::from_rgba(resized.into_raw(), 64, 64).ok()
}
