#[allow(unused_imports)]
use std::sync::Arc;
#[allow(unused_imports)]
use std::vec::IntoIter as VecIntoIter;
#[allow(unused_imports)]
use vulkano::descriptor::descriptor::DescriptorBufferDesc;
#[allow(unused_imports)]
use vulkano::descriptor::descriptor::DescriptorDesc;
#[allow(unused_imports)]
use vulkano::descriptor::descriptor::DescriptorDescTy;
#[allow(unused_imports)]
use vulkano::descriptor::descriptor::DescriptorImageDesc;
#[allow(unused_imports)]
use vulkano::descriptor::descriptor::DescriptorImageDescArray;
#[allow(unused_imports)]
use vulkano::descriptor::descriptor::DescriptorImageDescDimensions;
#[allow(unused_imports)]
use vulkano::descriptor::descriptor::ShaderStages;
#[allow(unused_imports)]
use vulkano::descriptor::descriptor_set::DescriptorSet;
#[allow(unused_imports)]
use vulkano::descriptor::descriptor_set::UnsafeDescriptorSet;
#[allow(unused_imports)]
use vulkano::descriptor::descriptor_set::UnsafeDescriptorSetLayout;
#[allow(unused_imports)]
use vulkano::descriptor::pipeline_layout::PipelineLayout;
#[allow(unused_imports)]
use vulkano::descriptor::pipeline_layout::PipelineLayoutDesc;
#[allow(unused_imports)]
use vulkano::descriptor::pipeline_layout::PipelineLayoutDescPcRange;
#[allow(unused_imports)]
use vulkano::device::Device;
#[allow(unused_imports)]
use vulkano::pipeline::shader::SpecializationConstants as SpecConstsTrait;
#[allow(unused_imports)]
use vulkano::pipeline::shader::SpecializationMapEntry;
pub struct Shader {
    shader: ::std::sync::Arc<::vulkano::pipeline::shader::ShaderModule>,
}
impl Shader {
    /// Loads the shader in Vulkan as a `ShaderModule`.
    #[inline]
    #[allow(unsafe_code)]
    pub fn load(
        device: ::std::sync::Arc<::vulkano::device::Device>,
    ) -> Result<Shader, ::vulkano::OomError> {
        let words = [
            119734787u32,
            66304u32,
            851975u32,
            32u32,
            0u32,
            131089u32,
            1u32,
            393227u32,
            1u32,
            1280527431u32,
            1685353262u32,
            808793134u32,
            0u32,
            196622u32,
            0u32,
            1u32,
            393231u32,
            5u32,
            4u32,
            1852399981u32,
            0u32,
            11u32,
            393232u32,
            4u32,
            17u32,
            64u32,
            1u32,
            1u32,
            196611u32,
            2u32,
            450u32,
            655364u32,
            1197427783u32,
            1279741775u32,
            1885560645u32,
            1953718128u32,
            1600482425u32,
            1701734764u32,
            1919509599u32,
            1769235301u32,
            25974u32,
            524292u32,
            1197427783u32,
            1279741775u32,
            1852399429u32,
            1685417059u32,
            1768185701u32,
            1952671090u32,
            6649449u32,
            262149u32,
            4u32,
            1852399981u32,
            0u32,
            196613u32,
            8u32,
            7890025u32,
            524293u32,
            11u32,
            1197436007u32,
            1633841004u32,
            1986939244u32,
            1952539503u32,
            1231974249u32,
            68u32,
            262149u32,
            17u32,
            1635017028u32,
            0u32,
            327686u32,
            17u32,
            0u32,
            1635017060u32,
            0u32,
            196613u32,
            19u32,
            6714722u32,
            262215u32,
            11u32,
            11u32,
            28u32,
            262215u32,
            16u32,
            6u32,
            4u32,
            327752u32,
            17u32,
            0u32,
            35u32,
            0u32,
            196679u32,
            17u32,
            2u32,
            262215u32,
            19u32,
            34u32,
            0u32,
            262215u32,
            19u32,
            33u32,
            0u32,
            262215u32,
            31u32,
            11u32,
            25u32,
            131091u32,
            2u32,
            196641u32,
            3u32,
            2u32,
            262165u32,
            6u32,
            32u32,
            0u32,
            262176u32,
            7u32,
            7u32,
            6u32,
            262167u32,
            9u32,
            6u32,
            3u32,
            262176u32,
            10u32,
            1u32,
            9u32,
            262203u32,
            10u32,
            11u32,
            1u32,
            262187u32,
            6u32,
            12u32,
            0u32,
            262176u32,
            13u32,
            1u32,
            6u32,
            196637u32,
            16u32,
            6u32,
            196638u32,
            17u32,
            16u32,
            262176u32,
            18u32,
            12u32,
            17u32,
            262203u32,
            18u32,
            19u32,
            12u32,
            262165u32,
            20u32,
            32u32,
            1u32,
            262187u32,
            20u32,
            21u32,
            0u32,
            262187u32,
            6u32,
            23u32,
            5u32,
            262176u32,
            24u32,
            12u32,
            6u32,
            262187u32,
            6u32,
            29u32,
            64u32,
            262187u32,
            6u32,
            30u32,
            1u32,
            393260u32,
            9u32,
            31u32,
            29u32,
            30u32,
            30u32,
            327734u32,
            2u32,
            4u32,
            0u32,
            3u32,
            131320u32,
            5u32,
            262203u32,
            7u32,
            8u32,
            7u32,
            327745u32,
            13u32,
            14u32,
            11u32,
            12u32,
            262205u32,
            6u32,
            15u32,
            14u32,
            196670u32,
            8u32,
            15u32,
            262205u32,
            6u32,
            22u32,
            8u32,
            393281u32,
            24u32,
            25u32,
            19u32,
            21u32,
            22u32,
            262205u32,
            6u32,
            26u32,
            25u32,
            327812u32,
            6u32,
            27u32,
            26u32,
            23u32,
            393281u32,
            24u32,
            28u32,
            19u32,
            21u32,
            22u32,
            196670u32,
            28u32,
            27u32,
            65789u32,
            65592u32,
        ];
        unsafe {
            Ok(Shader {
                shader: ::vulkano::pipeline::shader::ShaderModule::from_words(device, &words)?,
            })
        }
    }
    /// Returns the module that was created.
    #[allow(dead_code)]
    #[inline]
    pub fn module(&self) -> &::std::sync::Arc<::vulkano::pipeline::shader::ShaderModule> {
        &self.shader
    }
    /// Returns a logical struct describing the entry point named `{ep_name}`.
    #[inline]
    #[allow(unsafe_code)]
    pub fn main_entry_point(&self) -> ::vulkano::pipeline::shader::ComputeEntryPoint<(), Layout> {
        unsafe {
            #[allow(dead_code)]
            static NAME: [u8; 5usize] = [109u8, 97u8, 105u8, 110u8, 0];
            self.shader.compute_entry_point(
                ::std::ffi::CStr::from_ptr(NAME.as_ptr() as *const _),
                Layout(ShaderStages {
                    compute: true,
                    ..ShaderStages::none()
                }),
            )
        }
    }
}
pub struct MainInput;
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for MainInput {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            MainInput => {
                let mut debug_trait_builder = f.debug_tuple("MainInput");
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for MainInput {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for MainInput {
    #[inline]
    fn clone(&self) -> MainInput {
        {
            *self
        }
    }
}

#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::hash::Hash for MainInput {
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
        match *self {
            MainInput => {}
        }
    }
}
#[allow(unsafe_code)]
unsafe impl ::vulkano::pipeline::shader::ShaderInterfaceDef for MainInput {
    type Iter = MainInputIter;
    fn elements(&self) -> MainInputIter {
        MainInputIter { num: 0 }
    }
}
pub struct MainInputIter {
    num: u16,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for MainInputIter {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            MainInputIter {
                num: ref __self_0_0,
            } => {
                let mut debug_trait_builder = f.debug_struct("MainInputIter");
                let _ = debug_trait_builder.field("num", &&(*__self_0_0));
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for MainInputIter {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for MainInputIter {
    #[inline]
    fn clone(&self) -> MainInputIter {
        {
            *self
        }
    }
}
impl Iterator for MainInputIter {
    type Item = ::vulkano::pipeline::shader::ShaderInterfaceDefEntry;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        None
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = 0usize - self.num as usize;
        (len, Some(len))
    }
}
impl ExactSizeIterator for MainInputIter {}
pub struct MainOutput;
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for MainOutput {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            MainOutput => {
                let mut debug_trait_builder = f.debug_tuple("MainOutput");
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for MainOutput {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for MainOutput {
    #[inline]
    fn clone(&self) -> MainOutput {
        {
            *self
        }
    }
}

#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for MainOutput {
    #[inline]
    fn eq(&self, other: &MainOutput) -> bool {
        match *other {
            MainOutput => match *self {
                MainOutput => true,
            },
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for MainOutput {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {}
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::hash::Hash for MainOutput {
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
        match *self {
            MainOutput => {}
        }
    }
}
#[allow(unsafe_code)]
unsafe impl ::vulkano::pipeline::shader::ShaderInterfaceDef for MainOutput {
    type Iter = MainOutputIter;
    fn elements(&self) -> MainOutputIter {
        MainOutputIter { num: 0 }
    }
}
pub struct MainOutputIter {
    num: u16,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for MainOutputIter {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            MainOutputIter {
                num: ref __self_0_0,
            } => {
                let mut debug_trait_builder = f.debug_struct("MainOutputIter");
                let _ = debug_trait_builder.field("num", &&(*__self_0_0));
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for MainOutputIter {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for MainOutputIter {
    #[inline]
    fn clone(&self) -> MainOutputIter {
        {
            *self
        }
    }
}
impl Iterator for MainOutputIter {
    type Item = ::vulkano::pipeline::shader::ShaderInterfaceDefEntry;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        None
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = 0usize - self.num as usize;
        (len, Some(len))
    }
}
impl ExactSizeIterator for MainOutputIter {}
pub mod ty {
    #[repr(C)]
    #[allow(non_snake_case)]
    pub struct Data {
        pub data: [u32],
    }
}
pub struct Layout(pub ShaderStages);
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for Layout {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            Layout(ref __self_0_0) => {
                let mut debug_trait_builder = f.debug_tuple("Layout");
                let _ = debug_trait_builder.field(&&(*__self_0_0));
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for Layout {
    #[inline]
    fn clone(&self) -> Layout {
        match *self {
            Layout(ref __self_0_0) => Layout(::core::clone::Clone::clone(&(*__self_0_0))),
        }
    }
}
#[allow(unsafe_code)]
unsafe impl PipelineLayoutDesc for Layout {
    fn num_sets(&self) -> usize {
        1usize
    }
    fn num_bindings_in_set(&self, set: usize) -> Option<usize> {
        match set {
            0usize => Some(1usize),
            _ => None,
        }
    }
    fn descriptor(&self, set: usize, binding: usize) -> Option<DescriptorDesc> {
        match (set, binding) {
            (0usize, 0usize) => Some(DescriptorDesc {
                ty: DescriptorDescTy::Buffer(DescriptorBufferDesc {
                    dynamic: Some(false),
                    storage: true,
                }),
                array_count: 1u32,
                stages: self.0.clone(),
                readonly: true,
            }),
            _ => None,
        }
    }
    fn num_push_constants_ranges(&self) -> usize {
        0usize
    }
    fn push_constants_range(&self, num: usize) -> Option<PipelineLayoutDescPcRange> {
        if num != 0 || 0usize == 0 {
            None
        } else {
            Some(PipelineLayoutDescPcRange {
                offset: 0,
                size: 0usize,
                stages: ShaderStages::all(),
            })
        }
    }
}
#[allow(non_snake_case)]
#[repr(C)]
pub struct SpecializationConstants {}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(non_snake_case)]
impl ::core::fmt::Debug for SpecializationConstants {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            SpecializationConstants {} => {
                let mut debug_trait_builder = f.debug_struct("SpecializationConstants");
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(non_snake_case)]
impl ::core::marker::Copy for SpecializationConstants {}
#[automatically_derived]
#[allow(unused_qualifications)]
#[allow(non_snake_case)]
impl ::core::clone::Clone for SpecializationConstants {
    #[inline]
    fn clone(&self) -> SpecializationConstants {
        {
            *self
        }
    }
}
impl Default for SpecializationConstants {
    fn default() -> SpecializationConstants {
        SpecializationConstants {}
    }
}
unsafe impl SpecConstsTrait for SpecializationConstants {
    fn descriptors() -> &'static [SpecializationMapEntry] {
        static DESCRIPTORS: [SpecializationMapEntry; 0usize] = [];
        &DESCRIPTORS
    }
}
