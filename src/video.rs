use std::{
    collections::VecDeque,
    io::Read,
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Rgba8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    pub timestamp_micros: u64,
    pub pixels: Vec<u8>,
}

impl VideoFrame {
    pub fn rgba8(
        width: u32,
        height: u32,
        timestamp_micros: u64,
        pixels: Vec<u8>,
    ) -> Result<Self, String> {
        let expected = width
            .checked_mul(height)
            .and_then(|size| size.checked_mul(4))
            .ok_or_else(|| "Video frame dimensions overflowed.".to_string())?
            as usize;
        if pixels.len() != expected {
            return Err(format!(
                "Invalid RGBA frame size: expected {expected} bytes, got {}.",
                pixels.len()
            ));
        }
        Ok(Self {
            width,
            height,
            format: PixelFormat::Rgba8,
            timestamp_micros,
            pixels,
        })
    }
}

#[derive(Debug)]
pub struct VideoFrameQueue {
    frames: VecDeque<VideoFrame>,
    capacity: usize,
}

impl VideoFrameQueue {
    pub fn new(capacity: usize) -> Result<Self, String> {
        if capacity == 0 {
            return Err("Video frame queue capacity must be greater than zero.".into());
        }
        Ok(Self {
            frames: VecDeque::with_capacity(capacity),
            capacity,
        })
    }

    pub fn push(&mut self, frame: VideoFrame) {
        if self.frames.len() == self.capacity {
            self.frames.pop_front();
        }
        self.frames.push_back(frame);
    }

    pub fn latest(&mut self) -> Option<VideoFrame> {
        let latest = self.frames.pop_back();
        self.frames.clear();
        latest
    }
}

#[derive(Debug, Default)]
pub struct VideoCompositor {
    current: Option<VideoFrame>,
}

pub struct FfmpegBackend {
    frames: Receiver<Result<VideoFrame, String>>,
}

impl FfmpegBackend {
    pub fn start(url: &str, width: u32, height: u32, fps: u32) -> Result<Self, String> {
        if width == 0 || height == 0 || fps == 0 {
            return Err("FFmpeg video dimensions and frame rate must be positive.".into());
        }
        let frame_size = width as usize * height as usize * 4;
        let url = url.to_string();
        let (sender, receiver) = mpsc::sync_channel(8);

        std::thread::Builder::new()
            .name("karpe-ffmpeg-video".into())
            .spawn(move || {
                let mut child = match Command::new("ffmpeg")
                    .args([
                        "-hide_banner",
                        "-loglevel",
                        "error",
                        "-i",
                        &url,
                        "-vf",
                        &format!("scale={width}:{height}"),
                        "-r",
                        &fps.to_string(),
                        "-f",
                        "rawvideo",
                        "-pix_fmt",
                        "rgba",
                        "pipe:1",
                    ])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .spawn()
                {
                    Ok(child) => child,
                    Err(error) => {
                        let _ = sender.send(Err(format!(
                            "Could not start FFmpeg. Install FFmpeg and add it to PATH: {error}"
                        )));
                        return;
                    }
                };

                let mut stdout = child.stdout.take().expect("FFmpeg stdout was piped");
                let mut frame_bytes = vec![0; frame_size];
                let mut timestamp = 0;
                loop {
                    if let Err(error) = read_exact_frame(&mut stdout, &mut frame_bytes) {
                        if error.kind() == std::io::ErrorKind::UnexpectedEof {
                            break;
                        }
                        let _ = sender.send(Err(format!("FFmpeg frame read failed: {error}")));
                        break;
                    }
                    let frame = VideoFrame::rgba8(width, height, timestamp, frame_bytes.clone());
                    if sender.send(frame).is_err() {
                        break;
                    }
                    timestamp += 1_000_000 / fps as u64;
                }
                let _ = child.wait();
            })
            .map_err(|error| format!("Could not create FFmpeg thread: {error}"))?;

        Ok(Self { frames: receiver })
    }

    pub fn poll(&self, queue: &mut VideoFrameQueue) -> Result<usize, String> {
        let mut received = 0;
        while let Ok(result) = self.frames.try_recv() {
            queue.push(result?);
            received += 1;
        }
        Ok(received)
    }
}

fn read_exact_frame<R: Read>(reader: &mut R, buffer: &mut [u8]) -> std::io::Result<()> {
    let mut offset = 0;
    while offset < buffer.len() {
        let read = reader.read(&mut buffer[offset..])?;
        if read == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "FFmpeg ended before a complete frame was read",
            ));
        }
        offset += read;
    }
    Ok(())
}

impl VideoCompositor {
    pub fn submit(&mut self, frame: VideoFrame) {
        self.current = Some(frame);
    }

    pub fn clear(&mut self) {
        self.current = None;
    }

    pub fn current(&self) -> Option<&VideoFrame> {
        self.current.as_ref()
    }

    pub fn composite_into_frame(
        &self,
        target: &mut [u8],
        target_width: u32,
        target_height: u32,
        x: u32,
        y: u32,
    ) -> Result<(), String> {
        let Some(video) = self.current.as_ref() else {
            return Ok(());
        };
        if video.format != PixelFormat::Rgba8 {
            return Err("Unsupported video pixel format.".into());
        }
        let target_len = target_width as usize * target_height as usize * 4;
        if target.len() < target_len {
            return Err("Target framebuffer is smaller than its declared dimensions.".into());
        }

        for source_y in 0..video.height {
            let destination_y = y + source_y;
            if destination_y >= target_height {
                break;
            }
            for source_x in 0..video.width {
                let destination_x = x + source_x;
                if destination_x >= target_width {
                    break;
                }
                let source_index = ((source_y * video.width + source_x) * 4) as usize;
                let destination_index =
                    ((destination_y * target_width + destination_x) * 4) as usize;
                target[destination_index..destination_index + 4]
                    .copy_from_slice(&video.pixels[source_index..source_index + 4]);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{VideoCompositor, VideoFrame, VideoFrameQueue};

    #[test]
    fn validates_and_composites_rgba_frames() {
        let frame = VideoFrame::rgba8(1, 1, 0, vec![10, 20, 30, 255]).unwrap();
        let mut compositor = VideoCompositor::default();
        compositor.submit(frame);
        let mut target = vec![0; 2 * 2 * 4];
        compositor
            .composite_into_frame(&mut target, 2, 2, 1, 1)
            .unwrap();
        assert_eq!(&target[12..16], &[10, 20, 30, 255]);
    }

    #[test]
    fn queue_drops_oldest_frames() {
        let mut queue = VideoFrameQueue::new(1).unwrap();
        queue.push(VideoFrame::rgba8(1, 1, 0, vec![1, 1, 1, 255]).unwrap());
        queue.push(VideoFrame::rgba8(1, 1, 1, vec![2, 2, 2, 255]).unwrap());
        assert_eq!(queue.latest().unwrap().timestamp_micros, 1);
    }
}
