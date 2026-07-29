use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, TrayIconBuilder, TrayIconEvent};

pub static SHOULD_QUIT: AtomicBool = AtomicBool::new(false);
pub static SHOULD_SHOW: AtomicBool = AtomicBool::new(false);
pub static SHOULD_STOP_TRAY: AtomicBool = AtomicBool::new(false);

fn build_menu() -> Option<Menu> {
    let show = MenuItem::new("Show Settings", true, None);
    let show_id = show.id().clone();
    let quit = MenuItem::new("Quit", true, None);
    let quit_id = quit.id().clone();

    let menu = Menu::new();
    if menu.append_items(&[&show, &quit]).is_err() {
        return None;
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

    Some(menu)
}

/// Linux needs a gtk main loop running on the thread that owns the tray
/// icon (this is how the underlying StatusNotifierItem/AppIndicator
/// protocol gets pumped) - see the tray-icon crate's own README.
#[cfg(target_os = "linux")]
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

            let icon = match load_icon() {
                Some(icon) => icon,
                None => return,
            };

            let menu = match build_menu() {
                Some(menu) => menu,
                None => return,
            };

            let tray_icon = match TrayIconBuilder::new()
                .with_icon(icon)
                .with_menu(Box::new(menu))
                .with_tooltip("SyncNotes")
                .build()
            {
                Ok(tray_icon) => tray_icon,
                Err(_) => return,
            };
            let _tray_icon = tray_icon;

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

/// Windows needs a native win32 message loop running on the thread that owns
/// the tray icon, same idea as the Linux gtk loop above. It doesn't need any
/// window of its own, so we reuse winit (already in the dependency tree via
/// eframe) purely to pump OS messages on a background thread; the actual
/// tray/menu clicks arrive through the event handlers set in `build_menu`,
/// not through this loop.
#[cfg(target_os = "windows")]
pub fn create_tray() -> Option<JoinHandle<()>> {
    use winit::application::ApplicationHandler;
    use winit::event::WindowEvent;
    use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
    use winit::platform::windows::EventLoopBuilderExtWindows;
    use winit::window::WindowId;

    struct TrayPump;

    impl ApplicationHandler for TrayPump {
        fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

        fn window_event(
            &mut self,
            _event_loop: &ActiveEventLoop,
            _window_id: WindowId,
            _event: WindowEvent,
        ) {
        }

        fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
            if SHOULD_QUIT.load(Ordering::Relaxed) || SHOULD_STOP_TRAY.load(Ordering::Relaxed) {
                event_loop.exit();
                return;
            }
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                std::time::Instant::now() + std::time::Duration::from_millis(200),
            ));
        }
    }

    SHOULD_QUIT.store(false, Ordering::Relaxed);
    SHOULD_SHOW.store(false, Ordering::Relaxed);
    SHOULD_STOP_TRAY.store(false, Ordering::Relaxed);

    let handle = thread::Builder::new()
        .name("tray".into())
        .spawn(move || {
            let icon = match load_icon() {
                Some(icon) => icon,
                None => return,
            };

            let menu = match build_menu() {
                Some(menu) => menu,
                None => return,
            };

            let tray_icon = match TrayIconBuilder::new()
                .with_icon(icon)
                .with_menu(Box::new(menu))
                .with_tooltip("SyncNotes")
                .build()
            {
                Ok(tray_icon) => tray_icon,
                Err(_) => return,
            };
            let _tray_icon = tray_icon;

            let event_loop = match EventLoop::builder().with_any_thread(true).build() {
                Ok(event_loop) => event_loop,
                Err(_) => return,
            };
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                std::time::Instant::now() + std::time::Duration::from_millis(200),
            ));
            let mut app = TrayPump;
            let _ = event_loop.run_app(&mut app);
        })
        .ok()?;

    Some(handle)
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn create_tray() -> Option<JoinHandle<()>> {
    None
}

fn load_icon() -> Option<Icon> {
    let img = image::load_from_memory(include_bytes!("../../syncnotes-icon.png")).ok()?;
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
