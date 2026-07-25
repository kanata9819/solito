# Solito Terminal Emulator

Solito is a small, native terminal emulator for Windows, built with Rust. It combines a GPU-rendered interface with PTY-backed shell sessions and uses Nushell by default.

![Solito running Nushell](docs/images/solito-terminal.jpg)

Solito uses `winit` for its native window, `wgpu` for rendering, `glyphon` for text, and `portable-pty` for shell sessions.

## Features

- GPU-rendered terminal text
- PTY-backed shell sessions
- Multiple tabs with terminal-style shortcuts
- Keyboard copy mode
- Configurable font and window backdrop

since 2026/04/15

## License

MIT licensed. See `LICENSE`.
