//! Configuration related structs and functions for renderer.

use std::path::PathBuf;
use winit::dpi::{LogicalSize, Size};

/// Configuration of content system, rendering and other aspects of the renderer.
#[derive(Clone)]
pub struct RendererConfiguration {
    pub fullscreen: bool,
    pub resolution: [u16; 2],
    pub gpu: usize,
    pub content_roots: Vec<PathBuf>,
}

impl<'a> Into<Size> for &'a RendererConfiguration {
    fn into(self) -> Size {
        Size::Logical(LogicalSize::new(
            self.resolution[0] as f64,
            self.resolution[1] as f64,
        ))
    }
}

// default development configuration
impl Default for RendererConfiguration {
    fn default() -> Self {
        Self {
            fullscreen: false,
            resolution: [1600, 900],
            gpu: 0,
            content_roots: vec![PathBuf::from(
                "C:\\Users\\Matej\\CLionProjects\\renderer\\assets\\target",
            )],
        }
    }
}
