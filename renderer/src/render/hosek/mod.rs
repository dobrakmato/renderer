//! [Hosek-Wilkie] sky model data & rendering.
//!
//! [Hosek-Wilkie]: https://cgg.mff.cuni.cz/projects/SkylightModelling/

use crate::render::hosek::dataset::{DATASETS_RGB, DATASETS_RGB_RAD};
use crate::render::pools::{UniformBufferPool, UniformBufferPoolError};
use crate::render::ubo::HosekWilkieParams;
use crate::render::vertex::NormalMappedVertex;
use crate::resources::mesh::IndexedMesh;
use cgmath::Vector3;
use std::sync::Arc;
use vulkano::command_buffer::{AutoCommandBufferBuilder, DynamicState};
use vulkano::descriptor::DescriptorSet;
use vulkano::device::Device;
use vulkano::pipeline::GraphicsPipelineAbstract;

mod dataset;

/// Descriptor set index used for sky data.
pub const SKY_DATA_UBO_DESCRIPTOR_SET: usize = 1;

/// Uniform buffer poll for sky data.
pub type SkyDataPool = UniformBufferPool<HosekWilkieParams>;

/// Sky object that can be renderer and contains parameters for
/// underlying Hosek-Wilkie sky model.
pub struct Sky {
    pool: SkyDataPool,
    mesh: Arc<IndexedMesh<NormalMappedVertex, u16>>,
    pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    pub sun_dir: Vector3<f32>,
    pub turbidity: f32,
    pub ground_albedo: Vector3<f32>,
}

impl Sky {
    /// Creates a new `Sky` with specified parameters. Provided pipeline should be the one
    /// that will be used to render the sky.
    pub fn new(
        mesh: Arc<IndexedMesh<NormalMappedVertex, u16>>,
        device: Arc<Device>,
        pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    ) -> Self {
        let layout = pipeline
            .descriptor_set_layout(SKY_DATA_UBO_DESCRIPTOR_SET)
            .unwrap()
            .clone();

        Self {
            pool: SkyDataPool::new(device, layout),
            mesh,
            pipeline,
            sun_dir: Vector3::new(0.0, 1.0, 0.0),
            turbidity: 1.0,
            ground_albedo: Vector3::new(0.0, 0.0, 0.0),
        }
    }

    /// Returns descriptor set that can be used for rendering in this frame. Returned
    /// `DescriptorSet` may or may not be cached from previous frame(s).
    fn sky_params_data(&self) -> Result<impl DescriptorSet + Send + Sync, UniformBufferPoolError> {
        // todo: implement caching
        let data = make_hosek_wilkie_params(self.sun_dir, self.turbidity, self.ground_albedo);
        self.pool.next(data)
    }

    /// Records draw commands for this skybox into specifid *command buffer*.
    pub fn draw(
        &self,
        dynamic_state: &DynamicState,
        frame_matrix_data: Arc<dyn DescriptorSet + Send + Sync>,
        cmd: &mut AutoCommandBufferBuilder,
    ) {
        let sky_data = self
            .sky_params_data()
            .expect("cannot create HosekWilkieParams for this frame");

        cmd.draw_indexed(
            self.pipeline.clone(),
            dynamic_state,
            vec![self.mesh.vertex_buffer().clone()],
            self.mesh.index_buffer().clone(),
            (frame_matrix_data, sky_data),
            (),
        )
        .expect("cannot draw Sky");
    }
}

#[rustfmt::skip]
fn evaluate_spline(dataset: &[f32], start: usize, stride: usize, value: f32) -> f32 {
    1.0 * (1.0 - value).powi(5) * dataset[start + 0 * stride] +
        5.0 * (1.0 - value).powi(4) * value.powi(1) * dataset[start + 1 * stride] +
        10.0 * (1.0 - value).powi(3) * value.powi(2) * dataset[start + 2 * stride] +
        10.0 * (1.0 - value).powi(2) * value.powi(3) * dataset[start + 3 * stride] +
        5.0 * (1.0 - value).powi(1) * value.powi(4) * dataset[start + 4 * stride] +
        1.0 * value.powi(5) * dataset[start + 5 * stride]
}

fn evaluate(dataset: &[f32], stride: usize, turbidity: f32, albedo: f32, sun_theta: f32) -> f32 {
    // splines are functions of elevation^1/3
    let elevation_k = (1.0 - sun_theta / std::f32::consts::FRAC_PI_2)
        .max(0.0)
        .powf(1.0 / 3.0);

    // table has values for turbidity 1..10
    let turbidity0 = (turbidity as usize).max(1).min(10);
    let turbidity1 = 10.min(turbidity0 + 1);
    let turbidity_k = (turbidity - turbidity0 as f32).max(0.0).min(1.0);

    let dataset_a0 = 0;
    let dataset_a1 = stride * 6 * 10;

    let a0t0 = evaluate_spline(
        dataset,
        dataset_a0 + stride * 6 * (turbidity0 - 1),
        stride,
        elevation_k,
    );
    let a1t0 = evaluate_spline(
        dataset,
        dataset_a1 + stride * 6 * (turbidity0 - 1),
        stride,
        elevation_k,
    );
    let a0t1 = evaluate_spline(
        dataset,
        dataset_a0 + stride * 6 * (turbidity1 - 1),
        stride,
        elevation_k,
    );
    let a1t1 = evaluate_spline(
        dataset,
        dataset_a1 + stride * 6 * (turbidity1 - 1),
        stride,
        elevation_k,
    );

    a0t0 * (1.0 - albedo) * (1.0 - turbidity_k)
        + a1t0 * albedo * (1.0 - turbidity_k)
        + a0t1 * (1.0 - albedo) * turbidity_k
        + a1t1 * albedo * turbidity_k
}

/// Creates a Hosek-Wilkie params from provided parameters.
///
/// This is directly ported from example C++ code.
fn make_hosek_wilkie_params(
    sun_dir: Vector3<f32>,
    turbidity: f32,
    albedo: Vector3<f32>,
) -> HosekWilkieParams {
    let sun_theta = sun_dir.y.max(0.0).min(1.0).acos();

    #[inline]
    fn e(start: usize, turbidity: f32, albedo: Vector3<f32>, sun_theta: f32) -> Vector3<f32> {
        Vector3::new(
            evaluate(&DATASETS_RGB[0][start..], 9, turbidity, albedo.x, sun_theta),
            evaluate(&DATASETS_RGB[1][start..], 9, turbidity, albedo.y, sun_theta),
            evaluate(&DATASETS_RGB[2][start..], 9, turbidity, albedo.z, sun_theta),
        )
    }

    HosekWilkieParams {
        a: e(0, turbidity, albedo, sun_theta),
        b: e(1, turbidity, albedo, sun_theta),
        c: e(2, turbidity, albedo, sun_theta),
        d: e(3, turbidity, albedo, sun_theta),
        e: e(4, turbidity, albedo, sun_theta),
        f: e(5, turbidity, albedo, sun_theta),
        g: e(6, turbidity, albedo, sun_theta),
        h: e(8, turbidity, albedo, sun_theta),
        i: e(7, turbidity, albedo, sun_theta),
        z: Vector3::new(
            evaluate(DATASETS_RGB_RAD[0], 1, turbidity, albedo.x, sun_theta),
            evaluate(DATASETS_RGB_RAD[1], 1, turbidity, albedo.y, sun_theta),
            evaluate(DATASETS_RGB_RAD[2], 1, turbidity, albedo.z, sun_theta),
        ),
        sun_direction: sun_dir,
        padding0: 0.0,
        padding1: 0.0,
        padding2: 0.0,
        padding3: 0.0,
        padding4: 0.0,
        padding5: 0.0,
        padding6: 0.0,
        padding7: 0.0,
        padding8: 0.0,
        padding9: 0.0,
    }
}
