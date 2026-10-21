use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::{
    io::Cursor,
    sync::mpsc::{self, Receiver},
    time::Duration,
};

pub struct AudioEngine {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    sink: Option<Sink>,
    pending: Option<Receiver<Result<Vec<u8>, String>>>,
    pub state: AudioState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioState {
    Unavailable(String),
    Idle,
    Loading(String),
    Playing(String),
    Paused(String),
    Error(String),
}

impl AudioEngine {
    pub fn new() -> Result<Self, String> {
        let (stream, handle) = OutputStream::try_default()
            .map_err(|error| format!("Audio output unavailable: {error}"))?;
        Ok(Self {
            _stream: stream,
            handle,
            sink: None,
            pending: None,
            state: AudioState::Idle,
        })
    }

    pub fn load_and_play(&mut self, url: &str) {
        let url = url.to_string();
        let (sender, receiver) = mpsc::channel();
        self.pending = Some(receiver);
        self.state = AudioState::Loading(url.clone());
        std::thread::spawn(move || {
            let result = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(20))
                .user_agent("KARPE1/0.1 media")
                .build()
                .and_then(|client| client.get(&url).send())
                .and_then(|response| response.error_for_status())
                .and_then(|response| response.bytes())
                .map(|bytes| bytes.to_vec())
                .map_err(|error| format!("Could not load audio {url}: {error}"));
            let _ = sender.send(result);
        });
    }

    pub fn poll(&mut self) {
        let Some(receiver) = self.pending.as_ref() else {
            return;
        };
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.pending = None;
                self.state = AudioState::Error("Audio loading stopped unexpectedly.".into());
                return;
            }
        };
        self.pending = None;

        match result {
            Ok(bytes) => match Decoder::new(Cursor::new(bytes)) {
                Ok(source) => match Sink::try_new(&self.handle) {
                    Ok(sink) => {
                        sink.append(source);
                        self.sink = Some(sink);
                        if let AudioState::Loading(url) = &self.state {
                            self.state = AudioState::Playing(url.clone());
                        }
                    }
                    Err(error) => {
                        self.state =
                            AudioState::Error(format!("Could not create audio sink: {error}"));
                    }
                },
                Err(error) => {
                    self.state = AudioState::Error(format!("Unsupported audio stream: {error}"));
                }
            },
            Err(error) => self.state = AudioState::Error(error),
        }
    }

    pub fn toggle_pause(&mut self) {
        let Some(sink) = self.sink.as_ref() else {
            return;
        };
        if sink.is_paused() {
            sink.play();
            if let AudioState::Paused(url) = &self.state {
                self.state = AudioState::Playing(url.clone());
            }
        } else {
            sink.pause();
            if let AudioState::Playing(url) = &self.state {
                self.state = AudioState::Paused(url.clone());
            }
        }
    }
}

pub fn find_audio_sources(node: &crate::html::DomNode, sources: &mut Vec<String>) {
    if node.tag == "audio" || node.tag == "source" {
        if let Some(src) = node.attributes.get("src") {
            if !src.is_empty() {
                sources.push(src.clone());
            }
        }
    }
    for child in &node.children {
        find_audio_sources(child, sources);
    }
}

pub fn find_video_sources(node: &crate::html::DomNode, sources: &mut Vec<String>) {
    if node.tag == "video" || node.tag == "source" {
        if let Some(src) = node.attributes.get("src") {
            if !src.is_empty() {
                sources.push(src.clone());
            }
        }
    }
    for child in &node.children {
        find_video_sources(child, sources);
    }
}

#[cfg(test)]
mod tests {
    use super::find_audio_sources;
    use crate::html::parse;

    #[test]
    fn finds_audio_and_source_urls() {
        let document = parse(r#"<audio src="one.ogg"><source src="two.wav"></audio>"#);
        let mut sources = Vec::new();
        find_audio_sources(&document, &mut sources);
        assert_eq!(sources, vec!["one.ogg", "two.wav"]);
    }
}
