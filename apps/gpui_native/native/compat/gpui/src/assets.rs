use crate::{DevicePixels, Pixels, Result, SharedString, Size, size};
use smallvec::SmallVec;

use image::{Delay, Frame};
use std::{
    borrow::Cow,
    fmt,
    hash::Hash,
    sync::atomic::{AtomicUsize, Ordering::SeqCst},
};

/// A source of assets for this app to use.
pub trait AssetSource: 'static + Send + Sync {
    /// Load the given asset from the source path.
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>>;

    /// List the assets at the given path.
    fn list(&self, path: &str) -> Result<Vec<SharedString>>;
}

impl AssetSource for () {
    fn load(&self, _path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(None)
    }

    fn list(&self, _path: &str) -> Result<Vec<SharedString>> {
        Ok(vec![])
    }
}

/// A unique identifier for the image cache
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ImageId(pub usize);

#[derive(PartialEq, Eq, Hash, Clone)]
#[expect(missing_docs)]
pub struct RenderImageParams {
    pub image_id: ImageId,
    pub frame_index: usize,
}

/// A cached and processed image, in BGRA format
pub struct RenderImage {
    /// The ID associated with this image
    pub id: ImageId,
    /// The scale factor of this image on render.
    pub(crate) scale_factor: f32,
    data: SmallVec<[Frame; 1]>,
}

impl PartialEq for RenderImage {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for RenderImage {}

impl RenderImage {
    /// Create a new image from the given data.
    pub fn new(data: impl Into<SmallVec<[Frame; 1]>>) -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

        Self {
            id: ImageId(NEXT_ID.fetch_add(1, SeqCst)),
            scale_factor: 1.0,
            data: data.into(),
        }
    }

    /// Convert this image into a byte slice.
    pub fn as_bytes(&self, frame_index: usize) -> Option<&[u8]> {
        self.data
            .get(frame_index)
            .map(|frame| frame.buffer().as_raw().as_slice())
    }

    /// Get the size of this image, in pixels.
    pub fn size(&self, frame_index: usize) -> Size<DevicePixels> {
        self.data
            .get(frame_index)
            .map(|frame| {
                let (width, height) = frame.buffer().dimensions();
                size(width.into(), height.into())
            })
            .unwrap_or_default()
    }

    // Linear filtering must clamp at the image edge, not at the shared atlas
    // texture edge. Replicate one texel around the upload; the painted tile
    // excludes this border, preserving image dimensions and UV interpolation.
    pub(crate) fn atlas_data(&self, frame_index: usize) -> Option<(Size<DevicePixels>, Vec<u8>)> {
        let frame = self.data.get(frame_index)?.buffer();
        let width = frame.width() as usize;
        let height = frame.height() as usize;
        if width == 0 || height == 0 {
            return None;
        }
        // At the texture limit an image already spans the atlas edge, whose
        // sampler clamps. Use only the remaining room for its replicated edge.
        let atlas_width = width + 16_384usize.saturating_sub(width).min(2);
        let atlas_height = height + 16_384usize.saturating_sub(height).min(2);
        let left = (atlas_width - width) / 2;
        let top = (atlas_height - height) / 2;
        let stride = atlas_width * 4;
        let mut padded = vec![0; stride * atlas_height];
        for row in 0..atlas_height {
            let source_row = row.saturating_sub(top).min(height - 1);
            let source = &frame.as_raw()[source_row * width * 4..(source_row + 1) * width * 4];
            let target = &mut padded[row * stride..(row + 1) * stride];
            let start = left * 4;
            target[start..start + source.len()].copy_from_slice(source);
            if left > 0 {
                target[..4].copy_from_slice(&source[..4]);
            }
            if atlas_width > left + width {
                target[stride - 4..].copy_from_slice(&source[source.len() - 4..]);
            }
        }
        Some((
            size(
                DevicePixels(atlas_width as i32),
                DevicePixels(atlas_height as i32),
            ),
            padded,
        ))
    }

    /// Get the size of this image, in pixels for display, adjusted for the scale factor.
    pub(crate) fn render_size(&self, frame_index: usize) -> Size<Pixels> {
        self.size(frame_index)
            .map(|v| (v.0 as f32 / self.scale_factor).into())
    }

    /// Get the delay of this frame from the previous
    pub fn delay(&self, frame_index: usize) -> Delay {
        self.data
            .get(frame_index)
            .map(|frame| frame.delay())
            .unwrap_or(Delay::from_numer_denom_ms(100, 1))
    }

    /// Get the number of frames for this image.
    pub fn frame_count(&self) -> usize {
        self.data.len()
    }
}

impl fmt::Debug for RenderImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImageData")
            .field("id", &self.id)
            .field("size", &self.data.first().map(|f| f.buffer().dimensions()))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smallvec::SmallVec;

    #[test]
    fn atlas_data_repeats_a_single_pixel_without_changing_display_size() {
        let pixel = [49, 120, 198, 255];
        let image = RenderImage::new(vec![Frame::new(image::RgbaImage::from_pixel(
            1,
            1,
            image::Rgba(pixel),
        ))]);
        let (size, bytes) = image.atlas_data(0).unwrap();
        assert_eq!(size, super::size(DevicePixels(3), DevicePixels(3)));
        assert_eq!(bytes, pixel.repeat(9));
        assert_eq!(image.size(0), super::size(DevicePixels(1), DevicePixels(1)));
        assert_eq!(image.as_bytes(0), Some(pixel.as_slice()));
        assert!(image.atlas_data(1).is_none());
    }

    #[test]
    fn atlas_data_preserves_corners_and_transparency_for_each_frame() {
        let a = [1, 2, 3, 255];
        let b = [4, 5, 6, 128];
        let c = [7, 8, 9, 0];
        let d = [10, 11, 12, 255];
        let frame = Frame::new(image::RgbaImage::from_raw(2, 2, [a, b, c, d].concat()).unwrap());
        let image = RenderImage::new(vec![frame.clone(), frame]);
        for index in 0..2 {
            let (size, bytes) = image.atlas_data(index).unwrap();
            assert_eq!(size, super::size(DevicePixels(4), DevicePixels(4)));
            assert_eq!(
                bytes,
                [a, a, b, b, a, a, b, b, c, c, d, d, c, c, d, d].concat()
            );
        }
    }

    #[test]
    fn atlas_data_keeps_images_at_the_texture_limit_renderable() {
        for (width, height) in [(16_383, 1), (16_384, 1), (1, 16_383), (1, 16_384)] {
            let pixel = image::Rgba([12, 34, 56, 78]);
            let image = RenderImage::new(vec![Frame::new(image::RgbaImage::from_pixel(
                width, height, pixel,
            ))]);
            let (size, bytes) = image.atlas_data(0).unwrap();
            assert!(size.width.0 <= 16_384 && size.height.0 <= 16_384);
            assert_eq!(
                bytes.len(),
                size.width.0 as usize * size.height.0 as usize * 4
            );
            assert!(bytes.chunks_exact(4).all(|chunk| chunk == pixel.0));
        }
    }

    #[test]
    fn empty_render_image_does_not_panic() {
        let image = RenderImage::new(SmallVec::new());
        assert_eq!(image.frame_count(), 0);
        assert_eq!(image.size(0), Size::default());
        assert_eq!(image.as_bytes(0), None);
        assert!(image.atlas_data(0).is_none());
        assert_eq!(image.render_size(0), Size::default());
        assert_eq!(image.delay(0), Delay::from_numer_denom_ms(100, 1));
        let _ = format!("{image:?}");
    }
}
