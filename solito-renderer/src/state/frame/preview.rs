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

fn save_frame(renderer: &mut Renderer, path: &Path) -> Vec<u8> {
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
            view_formats: &renderer.window_surface.config.view_formats,
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
    renderer.draw_to_texture(&mut encoder, &texture).unwrap();
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
    pixels
}

fn assert_text_coverage(renderer: &mut Renderer, pixels: &[u8]) {
    let width = renderer.window_surface.config.width as usize;
    let origin_y = TerminalView::terminal_row_y(0, renderer.terminal_view.config.line_height);
    let glyphs = &mut renderer.terminal_view.glyphs;
    let mut edge_pixels = 0;
    for run in glyphs
        .text_buffer
        .layout_runs()
        .filter(|run| !run.text.trim().is_empty())
    {
        // The fixture starts each row with an isolated glyph on the default background.
        let Some(glyph) = run.glyphs.first() else {
            continue;
        };
        let physical = glyph.physical((TerminalView::PADDING_X, origin_y), 1.0);
        let image = glyphs
            .swash_cache
            .get_image_uncached(&mut glyphs.font_system, physical.cache_key)
            .unwrap();
        assert_eq!(image.content, glyphon::SwashContent::Mask);
        let left = physical.x + image.placement.left;
        let top = run.line_y.round() as i32 + physical.y - image.placement.top;
        let color = glyph.color_opt.unwrap_or_else(|| {
            let [r, g, b, a] = ThemeColor::TEXT;
            Color::rgba(r, g, b, a)
        });
        let foreground = [color.r(), color.g(), color.b()];
        for (index, coverage) in image.data.into_iter().enumerate() {
            let x = left as usize + index % image.placement.width as usize;
            let y = top as usize + index / image.placement.width as usize;
            let actual = &pixels[(y * width + x) * 4..][..3];
            // The rasterizer's coverage must interpolate encoded color bytes directly.
            // Linear-light blending brightens these edge pixels and fails this check.
            for channel in 0..3 {
                let background = f32::from(ThemeColor::BACKGROUND[channel]);
                let alpha = f32::from(coverage) / 255.0;
                let expected = background + (f32::from(foreground[channel]) - background) * alpha;
                assert!(
                    (f32::from(actual[channel]) - expected).abs() <= 2.0,
                    "{:?}, {}px, pixel ({x}, {y}), coverage {coverage}: {actual:?}, expected channel {channel} = {expected}",
                    renderer.window_surface.config.format,
                    renderer.terminal_view.config.font_size,
                );
            }
            if (32..224).contains(&coverage) {
                edge_pixels += 1;
            }
        }
    }
    assert!(
        edge_pixels > 20,
        "must exercise partially covered glyph edges"
    );
    let empty_pixel = &pixels[(width * 100 + width - 20) * 4..][..4];
    assert_eq!(empty_pixel, ThemeColor::BACKGROUND);
}

fn verify_text_rasterization(renderer: &mut Renderer, output: &Path) {
    use crate::state::resources::RenderResources;
    // Exercise both channel orders and the non-sRGB surface fallback, including
    // the physical font sizes produced by 100%, 125%, and 150% display scales.
    for format in [
        wgpu::TextureFormat::Bgra8UnormSrgb,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        wgpu::TextureFormat::Bgra8Unorm,
        wgpu::TextureFormat::Rgba8Unorm,
    ] {
        renderer.window_surface.config.format = format;
        renderer.window_surface.config.view_formats = if format.is_srgb() {
            vec![format.remove_srgb_suffix()]
        } else {
            vec![]
        };
        for scale in [1.0, 1.25, 1.5] {
            let config = RendererConfig {
                window_backdrop: WindowBackdrop::None,
                font_size: 17.0 * scale,
                line_height: 20.0 * scale,
                ..Default::default()
            };
            // These are offscreen targets; no unsupported format is presented.
            let [r, g, b, a] = if format.is_srgb() {
                crate::util::color::srgb_to_linear_rgba(ThemeColor::BACKGROUND)
            } else {
                crate::util::color::rgba_to_f32(ThemeColor::BACKGROUND)
            };
            renderer.window_surface.clear_color = wgpu::Color {
                r: f64::from(r),
                g: f64::from(g),
                b: f64::from(b),
                a: f64::from(a),
            };
            renderer.render_resources =
                RenderResources::new(&renderer.gpu, &renderer.window_surface);
            renderer.terminal_view = TerminalView::new(
                &renderer.gpu.device,
                &renderer.gpu.queue,
                format,
                PhysicalSize::new(
                    renderer.window_surface.config.width,
                    renderer.window_surface.config.height,
                ),
                config,
                glyphon::FontSystem::new(),
            );
            let mut terminal = TerminalState::new(renderer.terminal_size());
            terminal.apply_terminal_output(
                concat!(
                    "Welcome to Nushell\r\n",
                    "\x1b[32mNushell repository\x1b[0m\r\n",
                    "\x1b[35mhttps://www.nushell.sh\x1b[0m\r\n",
                    "日本語の文字描画\r\n",
                )
                .as_bytes(),
            );
            renderer.set_terminal_snapshot(terminal.snapshot());
            let path = output.join(format!("text-preview-{format:?}-{scale}.png"));
            let pixels = save_frame(renderer, &path);
            assert_text_coverage(renderer, &pixels);
        }
    }
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
            verify_text_rasterization(&mut renderer, &output);
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
