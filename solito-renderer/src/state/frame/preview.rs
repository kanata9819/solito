//! Render the real pipelines into PNGs without opening an interactive shell.
use super::*;
use crate::{RendererConfig, TabBarSnapshot, WindowBackdrop};
use solito_terminal::TerminalState;
use std::{fs::File, io::BufWriter, path::Path, sync::Arc};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    platform::windows::EventLoopBuilderExtWindows,
    window::{Window, WindowId},
};

fn save_frame(renderer: &mut Renderer, path: &Path) {
    let width = renderer.window_surface.config.width;
    let height = renderer.window_surface.config.height;
    renderer
        .terminal_view
        .glyphs
        .viewport
        .update(&renderer.gpu.queue, Resolution { width, height });
    renderer.prepare_text().unwrap();
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = renderer
        .gpu
        .device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("Appearance preview"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: renderer.window_surface.config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
    let stride = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
        * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let readback = renderer.gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Appearance readback"),
        size: u64::from(stride) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = renderer.create_encoder();
    renderer
        .draw_to_view(&mut encoder, &texture.create_view(&Default::default()))
        .unwrap();
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(height),
            },
        },
        size,
    );
    renderer.gpu.queue.submit(Some(encoder.finish()));
    let (tx, rx) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| tx.send(result).unwrap());
    renderer
        .gpu
        .device
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();
    rx.recv().unwrap().unwrap();
    let mapped = readback.slice(..).get_mapped_range().unwrap();
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for row in mapped.chunks_exact(stride as usize) {
        for pixel in row[..(width * 4) as usize].as_chunks::<4>().0 {
            if matches!(
                renderer.window_surface.config.format,
                wgpu::TextureFormat::Bgra8UnormSrgb | wgpu::TextureFormat::Bgra8Unorm
            ) {
                pixels.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
            } else {
                pixels.extend_from_slice(pixel);
            }
        }
    }
    drop(mapped);
    readback.unmap();
    let mut encoder = png::Encoder::new(BufWriter::new(File::create(path).unwrap()), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&pixels)
        .unwrap();
}

#[test]
#[ignore = "requires a Windows desktop and GPU adapter; writes appearance PNGs to target"]
fn render_appearance_previews() {
    struct Preview {
        complete: bool,
    }
    impl ApplicationHandler for Preview {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            let window = Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes()
                            .with_inner_size(PhysicalSize::new(1000, 620))
                            .with_visible(false),
                    )
                    .unwrap(),
            );
            let config = RendererConfig {
                window_backdrop: WindowBackdrop::None,
                font_size: 17.0,
                line_height: 26.0,
                ..Default::default()
            };
            let mut renderer = pollster::block_on(Renderer::new(window, config)).unwrap();
            let mut terminal = TerminalState::new(renderer.terminal_size());
            terminal.apply_terminal_output(concat!(
                "\x1b[38;2;124;174;166m~\x1b[0m  \x1b[38;2;145;157;164mC:\\dev\\solito\x1b[0m\r\n",
                "\x1b[38;2;124;174;166m>\x1b[0m cargo test --workspace\r\n\r\n",
                "   \x1b[38;2;145;157;164mCompiling\x1b[0m solito v0.2.0\r\n",
                "    \x1b[38;2;124;174;166mFinished\x1b[0m test profile in 1.42s\r\n\r\n",
                "running 48 tests\r\n",
                "test terminal::wide_characters ... \x1b[38;2;124;174;166mok\x1b[0m\r\n",
                "test terminal::scroll_regions  ... \x1b[38;2;124;174;166mok\x1b[0m\r\n",
                "test terminal::resize_cursor   ... \x1b[38;2;124;174;166mok\x1b[0m\r\n\r\n",
                "\x1b[38;2;124;174;166mtest result: ok.\x1b[0m 48 passed; 0 failed\r\n\r\n",
                "\x1b[38;2;145;157;164m日本語の表示も、そのまま。\x1b[0m\r\n\r\n",
                "\x1b[38;2;124;174;166m>\x1b[0m "
            ).as_bytes());
            renderer.set_terminal_snapshot(terminal.snapshot());
            renderer.set_tab_bar(TabBarSnapshot::new(
                vec!["Nushell".into(), "Neovim".into(), "日本語".into()],
                0,
            ));
            let output = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target");
            save_frame(&mut renderer, &output.join("ui-preview-focused.png"));
            renderer.set_tab_hover(Some(1));
            save_frame(&mut renderer, &output.join("ui-preview-hover.png"));
            renderer.set_focused(false);
            save_frame(&mut renderer, &output.join("ui-preview-inactive.png"));
            renderer.set_focused(true);
            renderer.resize(PhysicalSize::new(360, 420), terminal.snapshot());
            renderer.set_tab_bar(TabBarSnapshot::new(
                vec!["日本語の長いタブタイトル".into(); 10],
                8,
            ));
            save_frame(&mut renderer, &output.join("ui-preview-narrow.png"));
            self.complete = true;
            event_loop.exit();
        }
        fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
    }
    let event_loop = EventLoop::builder().with_any_thread(true).build().unwrap();
    let mut preview = Preview { complete: false };
    event_loop.run_app(&mut preview).unwrap();
    assert!(preview.complete);
}
