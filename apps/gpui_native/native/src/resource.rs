use crate::atoms;
#[cfg(feature = "real-gpui")]
use crate::gpui;
use rustler::{Binary, NifResult, Term};
#[cfg(feature = "real-gpui")]
use std::sync::Arc;

include!("generated/resources.rs");

#[derive(Clone, Debug)]
pub(crate) enum ImageData {
    Raster(RasterData),
    Ref(String),
}

pub(crate) fn decode_resource_ref(term: Term) -> NifResult<String> {
    let resource = decode_resource_ref_data(term)?;

    gpui_core::resource::validate_raster_ref(&resource.resource_type, &resource.id)
        .map_err(resource_ref_error)?;

    Ok(resource.id)
}

fn resource_ref_error(error: gpui_core::resource::ResourceRefError) -> rustler::Error {
    use gpui_core::resource::ResourceRefError;

    let reason = match error {
        ResourceRefError::UnsupportedType => "unsupported_resource_type",
        ResourceRefError::InvalidId => "invalid_resource_id",
    };
    rustler::Error::Term(Box::new(reason))
}

impl RasterData {
    #[cfg(any(feature = "real-gpui", test))]
    fn core_raster(self) -> gpui_core::raster::Raster {
        gpui_core::raster::Raster {
            width: self.width,
            height: self.height,
            format: self.format,
            stride: self.stride,
            data: self.data,
        }
    }

    #[cfg(any(feature = "real-gpui", test))]
    fn render_pixels(self) -> Result<Vec<u8>, gpui_core::raster::RasterError> {
        let mut pixels = self.core_raster().into_rgba()?;
        // RenderImage's Frame uses image::RgbaImage as its container, but GPUI
        // uploads those bytes as BGRA. Public Raster payloads remain straight RGBA.
        for pixel in pixels.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
        Ok(pixels)
    }

    pub(crate) fn validate(&self) -> NifResult<()> {
        gpui_core::raster::Raster {
            width: self.width,
            height: self.height,
            format: self.format.clone(),
            stride: self.stride,
            data: self.data.clone(),
        }
        .validate()
        .map_err(raster_error)
    }

    #[cfg(feature = "real-gpui")]
    pub(crate) fn render(self) -> gpui::Img {
        use gpui::{img, RenderImage, Styled};
        use image::{Frame, RgbaImage};

        let width = self.width;
        let height = self.height;
        let data = self.render_pixels().unwrap_or_default();
        let image =
            RgbaImage::from_raw(width, height, data).unwrap_or_else(|| RgbaImage::new(1, 1));
        let render_image = Arc::new(RenderImage::new(vec![Frame::new(image)]));

        img(render_image).size_full()
    }
}

fn raster_error(error: gpui_core::raster::RasterError) -> rustler::Error {
    use gpui_core::raster::RasterError;

    let reason = match error {
        RasterError::InvalidDimensions => "invalid_raster_dimensions",
        RasterError::UnsupportedFormat => "unsupported_raster_format",
        RasterError::SizeOverflow => "raster_size_overflow",
        RasterError::InvalidStride => "invalid_raster_stride",
        RasterError::InvalidData => "invalid_raster_data",
    };
    rustler::Error::Term(Box::new(reason))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_public_rgba_to_render_image_bgra() {
        let raster = RasterData {
            width: 1,
            height: 1,
            format: "rgba8".into(),
            stride: None,
            data: vec![240, 80, 10, 128],
        };
        assert_eq!(raster.render_pixels().unwrap(), [10, 80, 240, 128]);
    }

    #[test]
    fn preserves_bgra_channels_and_removes_row_padding_for_rendering() {
        let raster = RasterData {
            width: 1,
            height: 2,
            format: "bgra8".into(),
            stride: Some(8),
            data: vec![10, 80, 240, 128, 0, 0, 0, 0, 40, 50, 60, 255],
        };
        assert_eq!(
            raster.render_pixels().unwrap(),
            [10, 80, 240, 128, 40, 50, 60, 255]
        );
    }

    #[test]
    fn maps_core_validation_errors_to_nif_errors() {
        let raster = RasterData {
            width: 0,
            height: 1,
            format: "rgba8".to_string(),
            stride: None,
            data: Vec::new(),
        };

        assert!(raster.validate().is_err());
    }
}
