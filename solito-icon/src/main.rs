use std::{fs, path::Path};

use anyhow::{Context, Result};
use resvg::{tiny_skia, usvg};

fn main() -> Result<()> {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../solito/assets");
    let source = fs::read(assets.join("solito-icon.svg")).context("read icon SVG")?;
    let tree =
        usvg::Tree::from_data(&source, &usvg::Options::default()).context("parse icon SVG")?;
    let mut icons = ico::IconDir::new(ico::ResourceType::Icon);

    for size in [16, 24, 32, 48, 64, 128, 256, 1024] {
        let image = render_icon(&tree, size)?;
        let png = fs::File::create(assets.join(format!("solito-icon-{size}.png")))?;
        image.write_png(png)?;

        if size == 64 {
            fs::write(assets.join("solito-icon-64.rgba"), image.rgba_data())?;
        }
        if size <= 256 {
            icons.add_entry(ico::IconDirEntry::encode_as_png(&image)?);
        }
    }

    icons.write(fs::File::create(assets.join("solito.ico"))?)?;
    println!("Generated PNG, RGBA and ICO assets in {}", assets.display());
    Ok(())
}

fn render_icon(tree: &usvg::Tree, size: u32) -> Result<ico::IconImage> {
    let mut pixmap = tiny_skia::Pixmap::new(size, size).context("allocate icon pixels")?;
    let scale = size as f32 / tree.size().width().max(tree.size().height());
    let transform = tiny_skia::Transform::from_row(
        scale,
        0.0,
        0.0,
        scale,
        (size as f32 - tree.size().width() * scale) / 2.0,
        (size as f32 - tree.size().height() * scale) / 2.0,
    );
    resvg::render(tree, transform, &mut pixmap.as_mut());

    // tiny-skia stores premultiplied colors; PNG and winit need straight RGBA.
    let rgba = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let color = pixel.demultiply();
            [color.red(), color.green(), color.blue(), color.alpha()]
        })
        .collect();
    Ok(ico::IconImage::from_rgba_data(size, size, rgba))
}
