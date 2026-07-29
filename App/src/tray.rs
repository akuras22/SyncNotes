use std::sync::atomic::{AtomicBool, Ordering};

pub static TRAY_RUNNING: AtomicBool = AtomicBool::new(false);

pub fn start_tray() {
    if TRAY_RUNNING.load(Ordering::Relaxed) {
        return;
    }
    TRAY_RUNNING.store(true, Ordering::Relaxed);
}
