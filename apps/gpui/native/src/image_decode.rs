const MAX_ENCODED_BYTES: usize = 100 * 1_024 * 1_024;
const MAX_IMAGE_DIMENSION: u32 = 16_384;
const MAX_DECODED_BYTES: u64 = 256 * 1_024 * 1_024;

pub fn decode(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), image::ImageError> {
    use image::{error::LimitError, error::LimitErrorKind, ImageError, ImageReader, Limits};
    use std::io::Cursor;

    if bytes.len() > MAX_ENCODED_BYTES {
        return Err(ImageError::Limits(LimitError::from_kind(
            LimitErrorKind::InsufficientMemory,
        )));
    }

    let mut reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    if reader.format().is_none() {
        return decode_svg(bytes);
    }
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
    limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
    limits.max_alloc = Some(MAX_DECODED_BYTES);
    reader.limits(limits);

    let image = reader.decode()?.into_rgba8();
    let width = image.width();
    let height = image.height();
    Ok((width, height, image.into_raw()))
}

fn decode_svg(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), image::ImageError> {
    use image::{error::LimitError, error::LimitErrorKind, ImageError};
    use resvg::{tiny_skia, usvg};
    use std::sync::{Arc, OnceLock};

    static FONTS: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    let fonts = FONTS.get_or_init(|| {
        let mut fonts = usvg::fontdb::Database::new();
        fonts.load_system_fonts();
        Arc::new(fonts)
    });
    let options = usvg::Options {
        fontdb: Arc::clone(fonts),
        image_href_resolver: usvg::ImageHrefResolver {
            // SVG images behave as self-contained images, rather than documents
            // with access to files relative to the application's working directory.
            resolve_string: Box::new(|_, _| None),
            ..usvg::ImageHrefResolver::default()
        },
        ..usvg::Options::default()
    };
    let text = std::str::from_utf8(bytes).map_err(invalid_svg)?;
    let tree = usvg::Tree::from_str(text, &options).map_err(invalid_svg)?;
    let size = tree.size();
    if size.width() > MAX_IMAGE_DIMENSION as f32
        || size.height() > MAX_IMAGE_DIMENSION as f32
        || size.width().ceil() as u64 * size.height().ceil() as u64 * 4 > MAX_DECODED_BYTES
    {
        return Err(ImageError::Limits(LimitError::from_kind(
            LimitErrorKind::InsufficientMemory,
        )));
    }
    let size = size.to_int_size();
    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height())
        .ok_or_else(|| invalid_svg("SVG allocation failed"))?;
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    // tiny-skia stores premultiplied channels; GPUI.Raster accepts straight RGBA.
    for pixel in pixmap.data_mut().chunks_exact_mut(4) {
        let alpha = u16::from(pixel[3]);
        if alpha != 0 && alpha != 255 {
            for channel in &mut pixel[..3] {
                *channel = ((u16::from(*channel) * 255 + alpha / 2) / alpha) as u8;
            }
        }
    }
    Ok((size.width(), size.height(), pixmap.take()))
}

fn invalid_svg(error: impl std::fmt::Display) -> image::ImageError {
    image::ImageError::IoError(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        error.to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_image_bytes() {
        assert!(decode(b"not an image").is_err());
    }

    #[test]
    fn decodes_svg_colors_and_straight_alpha() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="1"><path fill="#ff0000" d="M0 0h1v1H0z"/><path fill="#00ff00" fill-opacity="0.5" d="M1 0h1v1H1z"/></svg>"##;
        let (width, height, pixels) = decode(svg).expect("SVG should decode");
        assert_eq!((width, height), (2, 1));
        assert_eq!(pixels, [255, 0, 0, 255, 0, 255, 0, 128]);
    }

    #[test]
    fn uses_svg_viewbox_dimensions() {
        let svg = br#"<?xml version="1.0"?><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 4 2"><rect width="4" height="2" fill="blue"/></svg>"#;
        let (width, height, pixels) = decode(svg).expect("viewBox SVG should decode");
        assert_eq!((width, height), (4, 2));
        assert!(pixels.chunks_exact(4).all(|p| p == [0, 0, 255, 255]));
    }

    #[test]
    fn rejects_svg_dimensions_and_allocations_beyond_limits() {
        for (width, height) in [(MAX_IMAGE_DIMENSION + 1, 1), (16_384, 16_384)] {
            let svg = format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}"/>"#
            );
            assert!(decode(svg.as_bytes()).is_err());
        }
    }

    #[test]
    fn does_not_resolve_external_svg_images() {
        use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
        use std::io::Cursor;
        use std::time::{SystemTime, UNIX_EPOCH};

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("gpui-svg-{}-{nonce}.png", std::process::id()));
        let mut png = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(1, 1, Rgba([255, 0, 0, 255])))
            .write_to(&mut png, ImageFormat::Png)
            .expect("PNG fixture should encode");
        std::fs::write(&path, png.get_ref()).expect("PNG fixture should save");
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><image href="{}" width="1" height="1"/></svg>"#,
            path.display()
        );
        let decoded = decode(svg.as_bytes());
        std::fs::remove_file(path).expect("PNG fixture should be removed");
        let (_, _, pixels) = decoded.expect("SVG itself should decode");
        assert_eq!(pixels, [0, 0, 0, 0]);
    }

    #[test]
    fn preserves_svg_gradients_and_clipping() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="1"><defs><linearGradient id="g"><stop stop-color="#ff0000"/><stop offset="1" stop-color="#0000ff"/></linearGradient><clipPath id="c"><rect width="2" height="1"/></clipPath></defs><rect width="4" height="1" fill="url(#g)" clip-path="url(#c)"/></svg>"##;
        let (_, _, pixels) = decode(svg).expect("gradient SVG should decode");
        assert!(pixels[0] > pixels[2]);
        assert!(pixels[4] < pixels[0]);
        assert_eq!(&pixels[8..], [0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn rejects_images_beyond_the_dimension_limit() {
        use image::{DynamicImage, ImageFormat, RgbaImage};
        use std::io::Cursor;

        let image = RgbaImage::new(MAX_IMAGE_DIMENSION + 1, 1);
        let mut encoded = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(image)
            .write_to(&mut encoded, ImageFormat::Bmp)
            .expect("test image should encode");

        assert!(decode(encoded.get_ref()).is_err());
    }
}
