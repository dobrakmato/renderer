use crate::perf::CPUProfiler;
use crate::Img2BfParameters;
use bf::image::{Format, Image};
use bf::{save_bf_to_bytes, Container, File};
use image::dxt::{DXTEncoder, DXTVariant};
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, ImageBuffer, ImageError, Pixel};
use std::ops::{Deref, DerefMut};

// generate `Statistics` struct with `CPUProfiler`s
impl_stats_struct!(pub Statistics; load, vflip, channels, swizzle, mipmaps, dxt, save);

#[derive(Debug)]
pub enum Img2BfError {
    InvalidDimensions(u32, u32),
    InputImageError(ImageError),
    BlockCompressionError(ImageError),
    SerializationError(bf::Error),
    SaveIOError(std::io::Error),
    InvalidSwizzle(&'static str),
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

    /// Swizzles the channels in the image according to parameters.
    fn swizzle(&mut self, image: &mut DynamicImage) -> Result<(), Img2BfError> {
        measure_scope!(self.stats.swizzle);

        // return early if swizzle is no-op
        if self.params.destination_r.is_none()
            && self.params.destination_g.is_none()
            && self.params.destination_b.is_none()
            && self.params.destination_a.is_none()
        {
            return Ok(());
        }

        /// Inner generic function that actually performs swizzling. The `image` is an
        /// `ImageBuffer` that will be modified by the swizzle operation. Parameter
        /// `components_order` specifies order of channels in the specified image buffer.
        /// Parameter `swizzle` actually contains the swizzling mapping.
        fn swizzle_inner<P, Container>(
            image: &mut ImageBuffer<P, Container>,
            components_order: &[char],
            swizzle: &[Option<&str>],
        ) where
            P: Pixel + 'static,
            P::Subpixel: 'static,
            Container: Deref<Target = [P::Subpixel]> + DerefMut,
        {
            // returns the index of specified channel in buffer
            let index_of_channel = |channel| {
                components_order
                    .iter()
                    .position(|x| x == &channel)
                    .expect("invalid components order")
            };

            // unwraps the swizzle for component or uses
            // default identity swizzle
            let swizzle_unwrap = |opt: Option<&str>, idx| {
                opt.map(|x| x.chars().next().unwrap())
                    .unwrap_or(components_order[idx])
            };

            for p in image.pixels_mut() {
                let original = *p;

                for (idx, x) in p.channels_mut().iter_mut().enumerate() {
                    *x = original
                        .channels()
                        .iter()
                        .nth(index_of_channel(swizzle_unwrap(swizzle[idx], idx)))
                        .copied()
                        .expect("invalid swizzle");
                }
            }
        }

        let order_rgb = ['r', 'g', 'b'];
        let order_rgba = ['r', 'g', 'b', 'a'];
        let order_bgr = ['b', 'g', 'r'];
        let order_bgra = ['b', 'g', 'r', 'a'];

        let swizzle_rgb = [
            self.params.destination_r.as_deref(),
            self.params.destination_g.as_deref(),
            self.params.destination_b.as_deref(),
        ];

        let swizzle_rgba = [
            self.params.destination_r.as_deref(),
            self.params.destination_g.as_deref(),
            self.params.destination_b.as_deref(),
            self.params.destination_a.as_deref(),
        ];

        let swizzle_bgr = [
            self.params.destination_b.as_deref(),
            self.params.destination_g.as_deref(),
            self.params.destination_r.as_deref(),
        ];

        let swizzle_bgra = [
            self.params.destination_b.as_deref(),
            self.params.destination_g.as_deref(),
            self.params.destination_r.as_deref(),
            self.params.destination_a.as_deref(),
        ];

        match image {
            DynamicImage::ImageRgb8(t) => swizzle_inner(t, &order_rgb, &swizzle_rgb),
            DynamicImage::ImageRgba8(t) => swizzle_inner(t, &order_rgba, &swizzle_rgba),
            DynamicImage::ImageBgr8(t) => swizzle_inner(t, &order_bgr, &swizzle_bgr),
            DynamicImage::ImageBgra8(t) => swizzle_inner(t, &order_bgra, &swizzle_bgra),
            DynamicImage::ImageRgb16(t) => swizzle_inner(t, &order_rgb, &swizzle_rgb),
            DynamicImage::ImageRgba16(t) => swizzle_inner(t, &order_rgba, &swizzle_rgba),
            _ => {
                return Err(Img2BfError::InvalidSwizzle(
                    "swizzle unsupported for this DynamicImage variant",
                ));
            }
        }

        Ok(())
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
                self.params.mip_filter.unwrap_or(FilterType::Lanczos3),
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

    /// Sets channels specified in channels array to zero.
    fn clear_channels(image: &mut DynamicImage, channels: &[usize]) {
        match image {
            DynamicImage::ImageRgb8(t) => {
                for x in t.pixels_mut() {
                    for (idx, n) in x.0.iter_mut().enumerate() {
                        if channels.contains(&idx) {
                            *n = 0;
                        }
                    }
                }
            }
            DynamicImage::ImageRgba8(t) => {
                for x in t.pixels_mut() {
                    for (idx, n) in x.0.iter_mut().enumerate() {
                        if channels.contains(&idx) {
                            *n = 0;
                        }
                    }
                }
            }
            _ => panic!("clear_channels not implemented this DynamicImage variant"),
        }
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

        if tool.params.pack_normal_map {
            tool.params.destination_r = Some("r".to_string());
            tool.params.destination_g = Some("g".to_string());
            tool.params.destination_b = Some("b".to_string());
            tool.params.destination_a = Some("r".to_string());
        }

        let image = tool.load_image()?;
        let (width, height) = tool.extract_dimensions(&image)?;
        let image = tool.v_flip(image)?;
        let mut image = tool.convert_channels(image)?;

        tool.swizzle(&mut image)?;

        if tool.params.pack_normal_map {
            Img2Bf::clear_channels(&mut image, &[0, 2]);
        }

        let mipmaps = tool.generate_mipmaps(image)?;
        let payload = tool.build_payload(mipmaps)?;

        tool.save_bf_image(width, height, payload)?;

        Ok(tool.stats)
    }
}
