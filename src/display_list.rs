use crate::graphics::{draw_rect, draw_text_with_font};
use crate::image::RgbaImage;

#[derive(Debug, Clone)]
pub enum DisplayCommand {
    SolidRect {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        color: [u8; 4],
    },
    Text {
        x: i32,
        y: i32,
        text: String,
        size: i32,
        color: [u8; 4],
    },
    Image {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        image: RgbaImage,
    },
}

#[derive(Debug, Default, Clone)]
pub struct DisplayList {
    commands: Vec<DisplayCommand>,
}

impl DisplayList {
    pub fn push(&mut self, command: DisplayCommand) {
        self.commands.push(command);
    }

    pub fn paint(&self, frame: &mut [u8], width: i32, height: i32) {
        for command in &self.commands {
            match command {
                DisplayCommand::SolidRect {
                    x,
                    y,
                    width: rect_width,
                    height: rect_height,
                    color,
                } => draw_rect(
                    frame,
                    width,
                    height,
                    *x,
                    *y,
                    *rect_width,
                    *rect_height,
                    *color,
                ),
                DisplayCommand::Text {
                    x,
                    y,
                    text,
                    size,
                    color,
                } => draw_text_with_font(frame, width, height, *x, *y, text, *size, *color),
                DisplayCommand::Image {
                    x,
                    y,
                    width: dest_width,
                    height: dest_height,
                    image,
                } => {
                    draw_image(
                        frame,
                        width,
                        height,
                        *x,
                        *y,
                        *dest_width,
                        *dest_height,
                        image,
                    );
                }
            }
        }
    }
}

fn draw_image(
    frame: &mut [u8],
    target_width: i32,
    target_height: i32,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    image: &RgbaImage,
) {
    if width <= 0 || height <= 0 || image.width == 0 || image.height == 0 {
        return;
    }
    for dy in 0..height {
        for dx in 0..width {
            let px = x + dx;
            let py = y + dy;
            if px < 0 || py < 0 || px >= target_width || py >= target_height {
                continue;
            }
            let sx = (dx as u32 * image.width / width as u32).min(image.width - 1);
            let sy = (dy as u32 * image.height / height as u32).min(image.height - 1);
            let source = ((sy * image.width + sx) * 4) as usize;
            let target = ((py * target_width + px) * 4) as usize;
            frame[target..target + 4].copy_from_slice(&image.pixels[source..source + 4]);
        }
    }
}
