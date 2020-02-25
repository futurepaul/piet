use lyon::tessellation::geometry_builder::*;
use crate::{WgpuVertex, WgpuBrush};

pub struct RenderPipelines {
    solid_pipeline: wgpu::RenderPipeline,
    linear_gradient_pipeline: wgpu::RenderPipeline,
}

pub struct BatchList {
    batches: Vec<Batch>,
}

impl BatchList {
    pub fn new() -> BatchList {
        BatchList {
            batches: Vec::new(),
        }
    }

    pub fn request_mesh(&mut self, brush: &WgpuBrush) -> (&mut VertexBuffers<WgpuVertex, u32>, u32) {
        unimplemented!();
        // Do logic to figure out batch based on brush
        // match brush {
        //     Solid => // check if current batch valid, otherwise create new one?
        // }
        //
        // TODO: Setup initial batch?

        let batch = if self.batches.last().unwrap().is_compatible_with_brush(brush) {
            self.batches.last()
        } else {
            // new batch
        }

        // push proper primitive

        // return the buffer and primitive id
    }
}

pub struct GradientHandle;
pub struct SolidPrimitive;
pub struct LinearGradientPrimitive;

pub struct Batch {
    kind: BatchKind,
}

pub enum BatchKind {
    Solid(SolidBatch),
    LinearGradient(LinearGradientBatch),
}

impl BatchKind {
    fn is_compatible_with_brush(&self, brush: &WgpuBrush) -> bool {
        match self {
            BatchKind::Solid(..) => brush.is_solid(),
            BatchKind::LinearGradient(batch) => brush.as_gradient().gradient == batch.gradient,
        }
    }
}

pub struct SolidBatch {
    mesh: VertexBuffers<WgpuVertex, u32>,
    primitives: Vec<SolidPrimitive>,
}

pub struct LinearGradientBatch {
    mesh: VertexBuffers<WgpuVertex, u32>,
    primitives: Vec<LinearGradientPrimitive>,
    gradient: GradientHandle,
}
