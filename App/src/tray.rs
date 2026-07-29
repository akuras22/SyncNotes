use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

pub static SHOULD_QUIT: AtomicBool = AtomicBool::new(false);
pub static SHOULD_SHOW: AtomicBool = AtomicBool::new(false);
pub static SHOULD_STOP_TRAY: AtomicBool = AtomicBool::new(false);

pub fn create_tray() -> Option<JoinHandle<()>> {
    SHOULD_QUIT.store(false, Ordering::Relaxed);
    SHOULD_SHOW.store(false, Ordering::Relaxed);
    SHOULD_STOP_TRAY.store(false, Ordering::Relaxed);

    let handle = thread::Builder::new()
        .name("tray".into())
        .spawn(move || {
            gtk::glib::set_application_name("SyncNotes");

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

            MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
                if event.id == quit_id {
                    SHOULD_QUIT.store(true, Ordering::Relaxed);
                } else if event.id == show_id {
                    SHOULD_SHOW.store(true, Ordering::Relaxed);
                }
            }));

            TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
                if let TrayIconEvent::Click { .. } = event {
                    SHOULD_SHOW.store(true, Ordering::Relaxed);
                }
            }));

            let tray_icon = match TrayIconBuilder::new()
                .with_icon(icon.unwrap())
                .with_menu(Box::new(menu))
                .with_tooltip("SyncNotes")
                .build()
            {
                Ok(tray_icon) => tray_icon,
                Err(_) => return,
            };

            let _tray_icon: TrayIcon = tray_icon;

            gtk::glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
                if SHOULD_QUIT.load(Ordering::Relaxed) || SHOULD_STOP_TRAY.load(Ordering::Relaxed) {
                    gtk::main_quit();
                    gtk::glib::ControlFlow::Break
                } else {
                    gtk::glib::ControlFlow::Continue
                }
            });
            gtk::main();
        })
        .ok()?;

    Some(handle)
}

fn load_icon() -> Option<Icon> {
    let img = image::load_from_memory(include_bytes!("../../logo.png")).ok()?;
    let rgba = trim_transparent(img.to_rgba8());
    let (w, h) = rgba.dimensions();
    let target = 256;
    let scale = (target as f32 / w as f32).min(target as f32 / h as f32);
    let scaled_w = (w as f32 * scale).round().max(1.0) as u32;
    let scaled_h = (h as f32 * scale).round().max(1.0) as u32;
    let resized = image::imageops::resize(&rgba, scaled_w, scaled_h, image::imageops::FilterType::Lanczos3);

    let mut canvas = image::RgbaImage::from_pixel(target, target, image::Rgba([0, 0, 0, 0]));
    let offset_x = (target - scaled_w) / 2;
    let offset_y = (target - scaled_h) / 2;
    image::imageops::overlay(&mut canvas, &resized, offset_x.into(), offset_y.into());

    Icon::from_rgba(canvas.into_raw(), target, target).ok()
}

fn trim_transparent(rgba: image::RgbaImage) -> image::RgbaImage {
    let (width, height) = rgba.dimensions();
    let raw = rgba.as_raw();

    let mut left = width;
    let mut right = 0;
    let mut top = height;
    let mut bottom = 0;

    for y in 0..height {
        for x in 0..width {
            let alpha = raw[((y * width + x) * 4 + 3) as usize];
            if alpha > 0 {
                left = left.min(x);
                right = right.max(x);
                top = top.min(y);
                bottom = bottom.max(y);
            }
        }
    }

    if left > right || top > bottom {
        return rgba;
    }

    image::imageops::crop_imm(&rgba, left, top, right - left + 1, bottom - top + 1).to_image()
}