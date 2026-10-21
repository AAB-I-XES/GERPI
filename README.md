# Rust Browser — From Scratch

A learning-oriented browser engine written from scratch in Rust.

## Current milestone

- Native application window
- Custom browser UI
- Address bar
- Navigation buttons
- Custom HTML tokenizer/parser
- DOM tree
- Basic layout
- Custom software-style painting into a pixel framebuffer
- Embedded SIXTY TrueType font system with glyph shaping, metrics, antialiased rasterization, and framebuffer painting
- Basic CSS cascade for tag, class, id, and inline selectors
- Safe JavaScript DOM writes with `document.write` and `document.body.innerHTML`
- Asynchronous audio loading and playback for direct `<audio src="...">` resources
- GPU-video foundation with validated RGBA frame queues and compositor upload boundaries
- Display-list based painting with solid-rectangle and text commands
- Unicode-safe text wrapping with basic block layout sizing
- DOM helpers for text content, selector lookup, mutation, and HTML void elements
- Boa JavaScript bindings for `document.querySelector("body")` and `document.createElement()`
- PNG/JPEG/GIF/WebP decoding primitives and basic SVG rectangle parsing
- Extensible web-font manager for loading TTF/OTF-compatible font data
- Hit testing for layout boxes and activation hooks for links, buttons, and inputs
- Promise-returning JavaScript `fetch()` binding backed by the existing network loader
- Fetches same-origin and absolute external `<link rel="stylesheet">` resources
- Built-in demo page

This project intentionally does **not** embed Chromium, WebKit, Gecko, Electron, Tauri, or a system WebView.

## Run

```bash
cargo run
```

## Open-source repository hygiene

Before local development, optionally copy `.env.example` to `.env` and place
machine-specific settings there. `.env` is ignored and must never be committed.
The repository also ignores Rust build output, editor metadata, logs, local
runtime data, credentials, certificates, and private media. Keep the bundled
font under `assets/fonts/` because it is required by the application and is
intended to be versioned.

Before publishing changes, inspect the staged file list:

```bash
git status --short
git diff --cached --name-only
```

If a secret was ever staged, remove it from the repository history and rotate
it; adding it to `.gitignore` does not remove an already tracked file.

## Controls

- Click the address bar and type a URL.
- Enter a URL or hostname to load it over HTTPS. Enter normal words to search DuckDuckGo.
- Network requests run in the background so the window remains responsive.
- Click Reload to repeat the current network request.
- CSS support currently covers colors, backgrounds, font sizes, margins, display modes, and pixel dimensions, including fetched external stylesheets. Flexbox/Grid are represented in the style model but full standards-compliant layout is still in progress.
- The window is presented through `pixels`/wgpu. Page painting now goes through a display list and remains CPU rasterization. YouTube playback requires media elements, codecs, GPU compositing, and DRM integration and is not supported by this from-scratch milestone.
- FFmpeg network playback is wired through a raw RGBA frame pipeline. Install `ffmpeg` and add it to `PATH`; MSE, EME, DRM/Widevine, and YouTube remain unsupported.
- Press `Esc` to exit.

## Roadmap

1. Window and framebuffer
2. Text rendering
3. HTTP client and search
4. HTML tokenizer
5. DOM
6. CSS parser
7. Style calculation
8. Layout engine
9. Painting
10. Images
11. JavaScript
12. Tabs/history/bookmarks
13. Security sandboxing
