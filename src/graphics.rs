pub fn draw_rect(
    frame: &mut [u8],
    width: i32,
    height: i32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: [u8; 4],
) {
    let x0 = x.max(0);
    let y0 = y.max(0);
    let x1 = (x + w).min(width);
    let y1 = (y + h).min(height);

    if x0 >= x1 || y0 >= y1 {
        return;
    }

    for yy in y0..y1 {
        for xx in x0..x1 {
            let i = ((yy * width + xx) * 4) as usize;
            if i + 3 < frame.len() {
                frame[i..i + 4].copy_from_slice(&color);
            }
        }
    }
}

pub fn draw_text(
    frame: &mut [u8],
    width: i32,
    height: i32,
    x: i32,
    y: i32,
    text: &str,
    size: i32,
    color: [u8; 4],
) {
    draw_text_with_font(frame, width, height, x, y, text, size, color);
}

pub fn draw_text_with_font(
    frame: &mut [u8],
    width: i32,
    height: i32,
    x: i32,
    y: i32,
    text: &str,
    size: i32,
    color: [u8; 4],
) {
    if let Err(error) =
        crate::text::draw_text(frame, width, height, x, y + size, text, size as f32, color)
    {
        eprintln!("Text rendering error: {error}");
    }
}
