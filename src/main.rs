mod browser;
mod css;
mod display_list;
mod events;
mod graphics;
mod html;
mod image;
mod javascript;
mod layout;
mod media;
mod network;
mod text;
mod video;

use browser::Browser;
use pixels::{Pixels, SurfaceTexture};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};

struct App {
    window: Option<&'static Window>,
    pixels: Option<Pixels<'static>>,
    browser: Browser,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            pixels: None,
            browser: Browser::new(),
        }
    }

    fn redraw(&mut self) {
        let Some(pixels) = self.pixels.as_mut() else {
            return;
        };

        // Get the dimensions BEFORE borrowing the framebuffer mutably.
        // This avoids Rust's E0502 borrow-checker error.
        let width = pixels.texture().width();
        let height = pixels.texture().height();

        // Now safely borrow the framebuffer mutably.
        let frame = pixels.frame_mut();

        self.browser.render(frame, width, height);

        // Present the rendered frame.
        if let Err(err) = pixels.render() {
            eprintln!("Render error: {err}");
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Don't create the window twice.
        if self.window.is_some() {
            return;
        }

        let window_attributes = Window::default_attributes()
            .with_title("Rust Browser — From Scratch")
            .with_inner_size(LogicalSize::new(1100.0, 720.0))
            .with_min_inner_size(LogicalSize::new(800.0, 500.0));

        let window = match event_loop.create_window(window_attributes) {
            Ok(window) => Box::leak(Box::new(window)),
            Err(error) => {
                eprintln!("Failed to create window: {error}");
                event_loop.exit();
                return;
            }
        };

        let size = window.inner_size();

        let surface_texture = SurfaceTexture::new(size.width, size.height, &*window);

        let pixels = match Pixels::new(size.width, size.height, surface_texture) {
            Ok(pixels) => pixels,
            Err(error) => {
                eprintln!("Failed to create pixel surface: {error}");
                event_loop.exit();
                return;
            }
        };

        self.window = Some(window);
        self.pixels = Some(pixels);
        window.set_ime_allowed(true);

        // Draw the first frame immediately.
        if let Some(window) = self.window {
            window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            // ---------------------------------------------------------
            // Close window
            // ---------------------------------------------------------
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            // ---------------------------------------------------------
            // Keyboard input
            // ---------------------------------------------------------
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: Key::Named(NamedKey::Escape),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                event_loop.exit();
            }

            WindowEvent::KeyboardInput { event, .. } => {
                self.browser.handle_key(event);

                if let Some(window) = self.window {
                    window.request_redraw();
                }
            }

            WindowEvent::Ime(winit::event::Ime::Commit(text)) => {
                self.browser.insert_text(&text);
                if let Some(window) = self.window {
                    window.request_redraw();
                }
            }

            // ---------------------------------------------------------
            // Redraw
            // ---------------------------------------------------------
            WindowEvent::RedrawRequested => {
                self.redraw();
            }

            // ---------------------------------------------------------
            // Window resize
            // ---------------------------------------------------------
            WindowEvent::Resized(size) => {
                if let Some(pixels) = self.pixels.as_mut() {
                    if let Err(error) = pixels.resize_surface(size.width, size.height) {
                        eprintln!("Failed to resize surface: {error}");
                    }

                    if let Err(error) = pixels.resize_buffer(size.width, size.height) {
                        eprintln!("Failed to resize pixel buffer: {error}");
                    }
                }

                if let Some(window) = self.window {
                    window.request_redraw();
                }
            }

            // ---------------------------------------------------------
            // Mouse movement
            // ---------------------------------------------------------
            WindowEvent::CursorMoved { position, .. } => {
                self.browser
                    .mouse_move(position.x as f32, position.y as f32);

                if let Some(window) = self.window {
                    window.request_redraw();
                }
            }

            // ---------------------------------------------------------
            // Mouse buttons
            // ---------------------------------------------------------
            WindowEvent::MouseInput { state, button, .. } => {
                self.browser.mouse_input(state, button);

                if let Some(window) = self.window {
                    window.request_redraw();
                }
            }

            // ---------------------------------------------------------
            // Other window events
            // ---------------------------------------------------------
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if self.browser.poll_navigation() {
            if let Some(window) = self.window {
                window.request_redraw();
            }
        }
        self.browser.poll_media();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Rust Browser...");

    let event_loop = EventLoop::new()?;

    let mut app = App::new();

    event_loop.run_app(&mut app)?;

    Ok(())
}
