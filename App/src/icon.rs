use image::RgbaImage;

pub fn load_logo_rgba(size: u32) -> Option<(Vec<u8>, u32, u32)> {
    let img = image::load_from_memory(include_bytes!("../../logo.png")).ok()?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let dim = w.max(h);
    let mut square = RgbaImage::from_pixel(dim, dim, image::Rgba([0, 0, 0, 0]));
    let ox = (dim - w) / 2;
    let oy = (dim - h) / 2;
    for (px, py, pixel) in rgba.enumerate_pixels() {
        square.put_pixel(ox + px, oy + py, pixel);
    }
    let resized = image::imageops::resize(&square, size, size, image::imageops::FilterType::Lanczos3);
    let out = resized.to_rgba8();
    Some((out.as_raw().clone(), size, size))
}
