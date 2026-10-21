# Architecture

The browser is divided into small components:

```text
UI
 |
 Browser / Navigation
 |
 Networking
 |
 HTML -> DOM
 |
 CSS -> Style
 |
 Layout
 |
 Paint
 |
 Framebuffer / GPU
```

The first implementation keeps networking out of the critical path so the project can
establish its own rendering pipeline first.

## Design rule

Browser-engine behavior should be implemented by this project. Low-level OS/window
libraries may be used for window creation and presenting pixels.
