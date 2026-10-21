use crate::{
    graphics::{draw_rect, draw_text},
    html::DomNode,
    layout::{layout_document, LayoutBox},
};
use std::sync::mpsc::{self, Receiver};
use winit::{
    event::{ElementState, KeyEvent, MouseButton},
    keyboard::{Key, NamedKey},
};

pub struct Browser {
    address: String,
    editing_address: bool,
    cursor: usize,
    mouse_x: f32,
    mouse_y: f32,
    dom: DomNode,
    layout: LayoutBox,
    navigation: Option<Receiver<Result<(String, String), String>>>,
    status: String,
    stylesheet: crate::css::Stylesheet,
    audio: Option<crate::media::AudioEngine>,
    video: Option<crate::video::FfmpegBackend>,
    video_queue: crate::video::VideoFrameQueue,
    video_compositor: crate::video::VideoCompositor,
    events: crate::events::EventDispatcher,
}

impl Browser {
    pub fn new() -> Self {
        let html = r#"
            <html>
                <body>
                    <h1>Rust Browser</h1>
                    <p>This page is rendered by a browser engine built from scratch.</p>
                    <p>No Chromium. No WebView. Just Rust.</p>
                    <h2>Milestone 1</h2>
                    <p>Window + HTML + DOM + layout + framebuffer painting.</p>
                </body>
            </html>
        "#;

        let stylesheet = crate::css::Stylesheet::from_html(html);
        let dom = crate::html::parse(html);
        let layout = layout_document(&dom, 40.0, 100.0, 1020.0, &stylesheet);

        Self {
            address: "rust://welcome".into(),
            editing_address: false,
            cursor: 0,
            mouse_x: 0.0,
            mouse_y: 0.0,
            dom,
            layout,
            navigation: None,
            status: String::new(),
            stylesheet,
            audio: crate::media::AudioEngine::new().ok(),
            video: None,
            video_queue: crate::video::VideoFrameQueue::new(4).expect("valid video queue"),
            video_compositor: crate::video::VideoCompositor::default(),
            events: crate::events::EventDispatcher::default(),
        }
    }

    pub fn handle_key(&mut self, event: KeyEvent) {
        if event.state != ElementState::Pressed {
            return;
        }

        if !self.editing_address {
            if event.logical_key == Key::Named(NamedKey::Escape) {
                return;
            }
            return;
        }

        match event.logical_key {
            Key::Named(NamedKey::Enter) => {
                self.editing_address = false;
                if self.address.is_empty() {
                    self.address = "rust://welcome".into();
                    return;
                }
                self.start_navigation();
            }

            Key::Named(NamedKey::Backspace) => {
                if self.cursor > 0 {
                    let previous = self.previous_boundary(self.cursor);
                    self.address.drain(previous..self.cursor);
                    self.cursor = previous;
                }
            }

            Key::Named(NamedKey::ArrowLeft) => {
                self.cursor = self.previous_boundary(self.cursor);
            }
            Key::Named(NamedKey::ArrowRight) => {
                self.cursor = self.next_boundary(self.cursor);
            }
            Key::Character(text) => {
                self.insert_text(&text);
            }
            _ => {}
        }
    }

    pub fn insert_text(&mut self, text: &str) {
        if !self.editing_address {
            return;
        }
        let filtered: String = text.chars().filter(|ch| !ch.is_control()).collect();
        if filtered.is_empty() {
            return;
        }
        self.address.insert_str(self.cursor, &filtered);
        self.cursor += filtered.len();
    }

    fn previous_boundary(&self, cursor: usize) -> usize {
        self.address[..cursor]
            .char_indices()
            .last()
            .map_or(0, |(index, _)| index)
    }

    fn next_boundary(&self, cursor: usize) -> usize {
        self.address[cursor..]
            .char_indices()
            .nth(1)
            .map_or(self.address.len(), |(index, _)| cursor + index)
    }

    fn start_navigation(&mut self) {
        let input = self.address.trim().to_string();
        let (sender, receiver) = mpsc::channel();
        self.navigation = Some(receiver);
        self.status = format!("Loading {input}...");
        std::thread::spawn(move || {
            let result = crate::network::fetch(&input);
            let _ = sender.send(result);
        });
    }

    pub fn poll_navigation(&mut self) -> bool {
        let Some(receiver) = self.navigation.as_ref() else {
            return false;
        };
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return false,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.navigation = None;
                self.status = "Navigation stopped unexpectedly.".into();
                return true;
            }
        };
        self.navigation = None;
        match result {
            Ok((url, body)) => {
                self.address = url;
                let body = crate::javascript::execute(&body);
                self.stylesheet = crate::css::Stylesheet::from_html(&body);
                self.dom = crate::html::parse(&body);
                self.layout = layout_document(&self.dom, 40.0, 100.0, 1020.0, &self.stylesheet);
                if !has_visible_content(&self.dom) {
                    let message = if is_youtube_url(&self.address) {
                        "YouTube requires JavaScript, DOM APIs, video codecs, and encrypted media support. This from-scratch engine does not implement those APIs yet."
                    } else {
                        "This page did not provide renderable HTML text. It may require JavaScript, CSS layout, or browser APIs that are not implemented yet."
                    };
                    let fallback = format!(
                        "<html><body><h1>Page cannot be rendered yet</h1><p>{message}</p><p>Direct HTML pages and audio resources are supported.</p></body></html>"
                    );
                    self.stylesheet = crate::css::Stylesheet::default();
                    self.dom = crate::html::parse(&fallback);
                    self.layout = layout_document(&self.dom, 40.0, 100.0, 1020.0, &self.stylesheet);
                }
                let mut sources = Vec::new();
                crate::media::find_audio_sources(&self.dom, &mut sources);
                if let (Some(audio), Some(source)) = (self.audio.as_mut(), sources.first()) {
                    if source.starts_with("http://") || source.starts_with("https://") {
                        audio.load_and_play(source);
                    }
                }
                let mut video_sources = Vec::new();
                crate::media::find_video_sources(&self.dom, &mut video_sources);
                if let Some(source) = video_sources.first() {
                    if source.starts_with("http://") || source.starts_with("https://") {
                        match crate::video::FfmpegBackend::start(source, 640, 360, 30) {
                            Ok(video) => {
                                self.video = Some(video);
                                self.status = "Loading video through FFmpeg...".into();
                            }
                            Err(error) => self.status = error,
                        }
                    }
                }
                if self.video.is_none() {
                    self.status.clear();
                }
            }

            Err(error) => {
                self.status = error.clone();
                let error_page =
                    format!("<html><body><h1>Navigation error</h1><p>{error}</p></body></html>");
                self.stylesheet = crate::css::Stylesheet::default();
                self.dom = crate::html::parse(&error_page);
                self.layout = layout_document(&self.dom, 40.0, 100.0, 1020.0, &self.stylesheet);
            }
        }

        true
    }

    pub fn poll_media(&mut self) {
        if let Some(audio) = self.audio.as_mut() {
            audio.poll();
        }

        if let Some(video) = self.video.as_ref() {
            match video.poll(&mut self.video_queue) {
                Ok(received) if received > 0 => {
                    if let Some(frame) = self.video_queue.latest() {
                        self.video_compositor.submit(frame);
                    }
                }
                Ok(_) => {}
                Err(error) => self.status = error,
            }
        }
    }

    pub fn mouse_move(&mut self, x: f32, y: f32) {
        self.mouse_x = x;
        self.mouse_y = y;
    }

    pub fn mouse_input(&mut self, state: ElementState, button: MouseButton) {
        if state != ElementState::Pressed || button != MouseButton::Left {
            return;
        }

        // Address bar
        if self.mouse_y >= 18.0 && self.mouse_y <= 58.0 {
            self.editing_address = true;
            self.cursor = self.address.len();
        }

        // Reload button
        if self.mouse_x >= 1030.0 && self.mouse_y >= 18.0 && self.mouse_y <= 58.0 {
            self.editing_address = false;
            if self.address != "rust://welcome" {
                self.start_navigation();
            }

            if let Some(target) = self
                .layout
                .hit_test(self.mouse_x as i32, self.mouse_y as i32)
            {
                if target.tag == "button" || target.tag == "input" || target.tag == "a" {
                    self.status = format!("{} activated", target.tag);
                }
                for script in self.events.dispatch(&target.tag) {
                    self.status = format!("Event executed: {script}");
                }
            }
        }
    }

    pub fn render(&self, frame: &mut [u8], width: u32, height: u32) {
        let w = width as i32;
        let h = height as i32;

        for px in frame.chunks_exact_mut(4) {
            px.copy_from_slice(&[245, 247, 250, 255]);
        }

        // Toolbar
        draw_rect(frame, w, h, 0, 0, w, 72, [232, 235, 240, 255]);
        draw_rect(frame, w, h, 0, 72, w, 1, [205, 209, 216, 255]);

        draw_text(frame, w, h, 18, 30, "‹", 26, [50, 55, 65, 255]);
        draw_text(frame, w, h, 50, 30, "›", 26, [50, 55, 65, 255]);

        draw_rect(frame, w, h, 100, 18, 920, 40, [255, 255, 255, 255]);
        draw_text(frame, w, h, 116, 31, &self.address, 15, [40, 45, 55, 255]);

        if self.editing_address {
            let cursor_x = 116 + crate::text::measure_text(&self.address, 15.0);
            draw_rect(frame, w, h, cursor_x, 28, 2, 20, [30, 30, 35, 255]);
        }

        draw_rect(frame, w, h, 1030, 18, 50, 40, [255, 255, 255, 255]);
        draw_text(frame, w, h, 1042, 30, "↻", 20, [50, 55, 65, 255]);

        // Page
        draw_rect(frame, w, h, 20, 90, w - 40, h - 110, [255, 255, 255, 255]);
        if let Err(error) = self
            .video_compositor
            .composite_into_frame(frame, width, height, 40, 100)
        {
            eprintln!("Video compositing error: {error}");
        }
        if !self.status.is_empty() {
            draw_text(frame, w, h, 40, 84, &self.status, 12, [90, 95, 105, 255]);
        }
        let mut display_list = crate::display_list::DisplayList::default();
        self.layout.display_list(&mut display_list);
        display_list.paint(frame, w, h);
    }
}

fn has_visible_content(node: &DomNode) -> bool {
    if node.tag == "#text" && !node.text.trim().is_empty() {
        return true;
    }
    node.children.iter().any(has_visible_content)
}

fn is_youtube_url(url: &str) -> bool {
    let host = url
        .split("://")
        .nth(1)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    host == "youtube.com"
        || host.ends_with(".youtube.com")
        || host == "youtu.be"
        || host.ends_with(".youtu.be")
}
