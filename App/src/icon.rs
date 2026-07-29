use image::GenericImageView;

pub fn load_logo_rgba(size: u32) -> Option<(Vec<u8>, u32, u32)> {
    let img = image::load_from_memory(include_bytes!("../../logo.png")).ok()?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let dim = w.min(h);
    let x = (w - dim) / 2;
    let y = (h - dim) / 2;
    let cropped = rgba.view(x, y, dim, dim).to_image();
    let resized = image::imageops::resize(&cropped, size, size, image::imageops::FilterType::Lanczos3);
    Some((resized.as_raw().clone(), size, size))
}
