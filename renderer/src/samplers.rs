use std::sync::Arc;
use vulkano::device::Device;
use vulkano::sampler::{Filter, MipmapMode, Sampler, SamplerAddressMode, SamplerCreationError};

/// Struct holding all available sampler instances to the renderer.
pub struct Samplers {
    pub aniso_repeat: Arc<Sampler>,
}

impl Samplers {
    pub fn new(device: Arc<Device>) -> Result<Self, SamplerCreationError> {
        let aniso_repeat = Sampler::new(
            device,
            Filter::Linear,
            Filter::Linear,
            MipmapMode::Linear,
            SamplerAddressMode::Repeat,
            SamplerAddressMode::Repeat,
            SamplerAddressMode::Repeat,
            0.0,
            16.0,
            0.0,
            1000.0,
        )?;
        Ok(Self { aniso_repeat })
    }
}
