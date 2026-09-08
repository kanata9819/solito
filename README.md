# Solito

Solito is a terminal emulator I'm building in Rust, mainly for Windows. It's a learning project: I wanted to understand what happens between typing a key and seeing a character on screen.

![Solito running Nushell](docs/images/solito-terminal.jpg)

It has tabs, a keyboard copy mode, and settings for the font and window backdrop. Rendering runs on the GPU, with `winit` handling the window, `wgpu` and `glyphon` drawing the screen, and `portable-pty` connecting to the shell.

There's still work to do on terminal compatibility. Neovim has been particularly good at finding bugs.

## Running it

```sh
cargo run --release -p solito
```

## Benchmarks

With `just` installed, this builds Solito and runs an automated Neovim session to measure CPU and memory use:

```sh
just bench
```

See [solito-bench](solito-bench/README.md) for the plain Cargo commands and other workloads. The benchmark picks its own shell for the run; your saved configuration stays as it is.

## Changing the icon

Edit [`solito/assets/solito-icon.svg`](solito/assets/solito-icon.svg), then run:

```sh
just ico
```

That regenerates the icon files and builds Solito with them.

Started on April 15, 2026.

## License

MIT. See [LICENSE](LICENSE).
