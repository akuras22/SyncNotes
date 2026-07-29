use image::GenericImageView;

pub fn load_logo_rgba(size: u32) -> Option<(Vec<u8>, u32, u32)> {
    let img = image::load_from_memory(include_bytes!("../../logo.png")).ok()?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let scale = (size as f32 / w as f32).min(size as f32 / h as f32);
    let scaled_w = (w as f32 * scale).round().max(1.0) as u32;
    let scaled_h = (h as f32 * scale).round().max(1.0) as u32;
    let resized = image::imageops::resize(&rgba, scaled_w, scaled_h, image::imageops::FilterType::Lanczos3);

    let mut canvas = image::RgbaImage::from_pixel(size, size, image::Rgba([0, 0, 0, 0]));
    let offset_x = (size - scaled_w) / 2;
    let offset_y = (size - scaled_h) / 2;
    image::imageops::overlay(&mut canvas, &resized, offset_x.into(), offset_y.into());

    Some((canvas.into_raw(), size, size))
}
