pub fn load_logo_rgba(size: u32) -> Option<(Vec<u8>, u32, u32)> {
    let img = image::load_from_memory(include_bytes!("../../logo.png")).ok()?;
    let rgba = trim_transparent(img.to_rgba8());
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
