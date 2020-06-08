use crate::perf::CPUProfiler;
use crate::Img2BfParameters;
use bf::image::{Format, Image};
use bf::{save_bf_to_bytes, Container, File};
use image::dxt::{DXTEncoder, DXTVariant};
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, ImageError};

// generate `Statistics` struct with `CPUProfiler`s
impl_stats_struct!(pub Statistics; load, vflip, channels, mipmaps, dxt, save);

#[derive(Debug)]
pub enum Img2BfError {
    InvalidDimensions(u32, u32),
    InputImageError(ImageError),
    BlockCompressionError(ImageError),
    SerializationError(bf::Error),
    SaveIOError(std::io::Error),
}

pub struct Img2Bf {
    params: Img2BfParameters,
    stats: Statistics<'static>,
}

impl Img2Bf {
    /// Loads the image.
    fn load_image(&mut self) -> Result<DynamicImage, Img2BfError> {
        measure_scope!(self.stats.load);

        Ok(image::open(&self.params.input).map_err(Img2BfError::InputImageError)?)
    }

    /// Validates the dimensions of image and returns them as pair of `u16`.
    fn extract_dimensions(&self, image: &DynamicImage) -> Result<(u16, u16), Img2BfError> {
        let (width, height) = image.dimensions();

        if width > 65535 || height > 65535 {
            return Err(Img2BfError::InvalidDimensions(width, height));
        }

        Ok((width as u16, height as u16))
    }

    /// Vertically flips the image if requested via parameters.
    fn v_flip(&mut self, image: DynamicImage) -> Result<DynamicImage, Img2BfError> {
        measure_scope!(self.stats.vflip);

        if self.params.v_flip {
            Ok(image.flipv())
        } else {
            Ok(image)
        }
    }

    /// Converts the `DynamicImage` into correct channel form.
    fn convert_channels(&mut self, image: DynamicImage) -> Result<DynamicImage, Img2BfError> {
        measure_scope!(self.stats.channels);

        if image.color().channel_count() != self.params.format.channels() {
            match self.params.format.channels() {
                1 => Ok(DynamicImage::ImageLuma8(image.to_luma())),
                3 => Ok(DynamicImage::ImageRgb8(image.to_rgb())),
                4 => Ok(DynamicImage::ImageRgba8(image.to_rgba())),
                _ => panic!("requested output format has unsupported num of channels"),
            }
        } else {
            Ok(image)
        }
    }

    /// Generates a mip-maps and returns all images (including the
    /// highest resolution mip-map - the passed in `image`).
    fn generate_mipmaps(&mut self, image: DynamicImage) -> Result<Vec<DynamicImage>, Img2BfError> {
        measure_scope!(self.stats.mipmaps);

        let mut mipmaps = vec![image];

        // 4 is the minimal size for dxt texture
        while mipmaps.last().unwrap().width() > 4 {
            let higher = mipmaps.last().unwrap();
            let lower = higher.clone().resize(
                higher.width() / 2,
                higher.height() / 2,
                FilterType::Lanczos3,
            );
            mipmaps.push(lower);
        }

        Ok(mipmaps)
    }

    /// Performs the image block compression to specified `target_format`. Parameters
    /// `width` and `height` represent width and height of image data in parameter
    /// `raw`.
    ///
    /// Depending on the `target_format` best encoder will be used.
    fn compress_image(target_format: Format, image: &DynamicImage) -> Result<Vec<u8>, Img2BfError> {
        // image-rs dxt encoder function
        let image_dxt = |variant| {
            let mut storage: Vec<u8> = vec![];
            DXTEncoder::new(&mut storage)
                .encode(&image.to_bytes(), image.width(), image.height(), variant)
                .map_err(Img2BfError::BlockCompressionError)
                .map(|()| storage)
        };

        let rgba_image = image.to_rgba(); // todo: lazily evaluate
        let intel_tex_surface = || intel_tex::RgbaSurface {
            data: rgba_image.as_ref(),
            width: image.width(),
            height: image.height(),
            stride: image.width() * 4,
        };

        let intel_tex_bc6h =
            |settings| intel_tex::bc6h::compress_blocks(&settings, &intel_tex_surface());

        let intel_tex_bc7 =
            |settings| intel_tex::bc7::compress_blocks(&settings, &intel_tex_surface());

        // match the requested format and compress with best encoder for specified
        // format.
        let result = match target_format {
            Format::SrgbDxt1 | Format::Dxt1 => {
                intel_tex::bc1::compress_blocks(&intel_tex_surface())
            }
            Format::SrgbDxt3 | Format::Dxt3 => image_dxt(DXTVariant::DXT3)?,
            Format::SrgbDxt5 | Format::Dxt5 => {
                intel_tex::bc3::compress_blocks(&intel_tex_surface())
            }
            Format::BC7 => intel_tex_bc7(intel_tex::bc7::alpha_slow_settings()),
            Format::SrgbBC7 => intel_tex_bc7(intel_tex::bc7::opaque_slow_settings()),
            Format::BC6H => intel_tex_bc6h(intel_tex::bc6h::slow_settings()),
            _ => panic!(
                "Format {:?} is not compressed but `compress_image` was called.",
                target_format
            ),
        };

        Ok(result)
    }

    /// Builds the payload of specified mip-maps by:
    ///   1. compressing them with requested block compression algorithm
    ///   2. appending them to `Vec<u8>`
    /// The function returns the resulting payload.
    fn build_payload(&mut self, mipmaps: Vec<DynamicImage>) -> Result<Vec<u8>, Img2BfError> {
        measure_scope!(self.stats.dxt);

        let mut payload = vec![];
        for img in mipmaps {
            // if the target format is compressed we need to compress raw image
            // data before appending it to payload
            let result = if self.params.format.compressed() {
                Img2Bf::compress_image(self.params.format, &img)?
            } else {
                img.to_bytes()
            };

            payload.extend(result);
        }

        Ok(payload)
    }

    /// Saves the specified information into an BF file to path specified by
    /// parameters.
    fn save_bf_image(
        &mut self,
        width: u16,
        height: u16,
        payload: Vec<u8>,
    ) -> Result<(), Img2BfError> {
        measure_scope!(self.stats.save);

        let file = File::create_compressed(Container::Image(Image {
            width,
            height,
            format: self.params.format,
            mipmap_data: payload.as_slice(),
        }));

        let default_output = self.params.input.with_extension("bf");
        let save_path = self.params.output.clone().unwrap_or(default_output);
        let bytes = save_bf_to_bytes(&file).map_err(Img2BfError::SerializationError)?;

        std::fs::write(save_path, bytes).map_err(Img2BfError::SaveIOError)?;

        Ok(())
    }

    /// Calling this method performs the conversion specified by `Img2BfParameters` parameter.
    /// If the conversion is successful the `Statistics` object will be returned which
    /// contains statistic information about the conversion. Error will be returned otherwise.
    pub fn convert(params: Img2BfParameters) -> Result<Statistics<'static>, Img2BfError> {
        let mut tool = Img2Bf {
            params,
            stats: Statistics::default(),
        };

        let image = tool.load_image()?;
        let (width, height) = tool.extract_dimensions(&image)?;
        let image = tool.v_flip(image)?;
        let image = tool.convert_channels(image)?;
        let mipmaps = tool.generate_mipmaps(image)?;
        let payload = tool.build_payload(mipmaps)?;

        tool.save_bf_image(width, height, payload)?;

        Ok(tool.stats)
    }
}
