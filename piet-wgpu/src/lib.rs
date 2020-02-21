use std::borrow::Cow;
use std::fmt;

use piet::kurbo::{Affine, PathEl, Point, QuadBez, Rect, Shape};

use piet::{
    new_error, Color, Error, ErrorKind, FixedGradient, ImageFormat, InterpolationMode, IntoBrush,
    LineCap, LineJoin, RenderContext, StrokeStyle,
};

use lyon::math::Point;
use lyon::path::PathEvent;
use lyon::tessellation::geometry_builder::*;
use lyon::tessellation::{self, FillOptions, FillTessellator, StrokeOptions, StrokeTessellator};

struct PietVertex {
  pos: [f32; 2]
}

struct LyonCtx {
  fill_tess: FillTessellator,
  stroke_tess: StrokeTessellator,
  mesh: VertexBuffers<PietVertex, u32>,

  // transforms: Vec<GpuTransform>
  // primitives: 
}

struct WgpuCtx<'a> {
  device: &wgpu::Device,
}


pub struct WgpuRenderContext {
  lyon_ctx: LyonCtx,
  wgpu_ctx: WgpuCtx,
}

pub trait RenderContext
where
    Self::Brush: IntoBrush<Self>,
{
    /// The type of a "brush".
    ///
    /// Represents solid colors and gradients.
    type Brush: Clone;

    /// An associated factory for creating text layouts and related resources.
    type Text: Text<TextLayout = Self::TextLayout>;
    type TextLayout: TextLayout;

    /// The associated type of an image.
    type Image;

    /// Report an internal error.
    ///
    /// Drawing operations may cause internal errors, which may also occur
    /// asynchronously after the drawing command was issued. This method reports
    /// any such error that has been detected.
    fn status(&mut self) -> Result<(), Error> {
      unimplemented!()
    }

    /// Create a new brush resource.
    ///
    /// TODO: figure out how to document lifetime and rebuilding requirements. Should
    /// that be the responsibility of the client, or should the back-end take
    /// responsibility? We could have a cache that is flushed when the Direct2D
    /// render target is rebuilt. Solid brushes are super lightweight, but
    /// other potentially retained objects will be heavier.
    fn solid_brush(&mut self, color: Color) -> Self::Brush {
      unimplemented!()
    }

    /// Create a new gradient brush.
    fn gradient(&mut self, gradient: impl Into<FixedGradient>) -> Result<Self::Brush, Error> {
      unimplemented!()
    }

    /// Clear the canvas with the given color.
    ///
    /// Note: only opaque colors are meaningful.
    fn clear(&mut self, color: Color) {
      unimplemented!()
    }

    /// Stroke a shape.
    fn stroke(&mut self, shape: impl Shape, brush: &impl IntoBrush<Self>, width: f64) {
      unimplemented!()
    }

    /// Stroke a shape, with styled strokes.
    fn stroke_styled(
        &mut self,
        shape: impl Shape,
        brush: &impl IntoBrush<Self>,
        width: f64,
        style: &StrokeStyle,
    ) {
      unimplemented!()
    }

    /// Fill a shape, using non-zero fill rule.
    fn fill(&mut self, shape: impl Shape, brush: &impl IntoBrush<Self>) {
      unimplemented!()
    }

    /// Fill a shape, using even-odd fill rule
    fn fill_even_odd(&mut self, shape: impl Shape, brush: &impl IntoBrush<Self>) {
      unimplemented!()
    }

    /// Clip to a shape.
    ///
    /// All subsequent drawing operations up to the next [`restore`](#method.restore)
    /// are clipped by the shape.
    fn clip(&mut self, shape: impl Shape) {
      unimplemented!()
    }

    fn text(&mut self) -> &mut Self::Text {
      unimplemented!()
    }

    /// Draw a text layout.
    ///
    /// The `pos` parameter specifies the baseline of the left starting place of
    /// the text. Note: this is true even if the text is right-to-left.
    fn draw_text(
        &mut self,
        layout: &Self::TextLayout,
        pos: impl Into<Point>,
        brush: &impl IntoBrush<Self>,
    ) {
      unimplemented!()
    }

    /// Save the context state.
    ///
    /// Pushes the current context state onto a stack, to be popped by
    /// [`restore`](#method.restore).
    ///
    /// Prefer [`with_save`](#method.with_save) if possible, as that statically
    /// enforces balance of save/restore pairs.
    ///
    /// The context state currently consists of a clip region and an affine
    /// transform, but is expected to grow in the near future.
    fn save(&mut self) -> Result<(), Error> {
      unimplemented!()
    }

    /// Restore the context state.
    ///
    /// Pop a context state that was pushed by [`save`](#method.save). See
    /// that method for details.
    fn restore(&mut self) -> Result<(), Error> {
      unimplemented!()
    }

    /// Finish any pending operations.
    ///
    /// This will generally be called by a shell after all user drawing
    /// operations but before presenting. Not all back-ends will handle this
    /// the same way.
    fn finish(&mut self) -> Result<(), Error> {
      unimplemented!()
    }

    /// Apply a transform.
    ///
    /// Apply an affine transformation. The transformation remains in effect
    /// until a [`restore`](#method.restore) operation.
    fn transform(&mut self, transform: Affine) {
      unimplemented!()
    }

    /// Create a new image from a pixel buffer.
    fn make_image(
        &mut self,
        width: usize,
        height: usize,
        buf: &[u8],
        format: ImageFormat,
    ) -> Result<Self::Image, Error> {
      unimplemented!()
    }

    /// Draw an image.
    ///
    /// The image is scaled to the provided `rect`. It will be squashed if
    /// aspect ratios don't match.
    fn draw_image(&mut self, image: &Self::Image, rect: impl Into<Rect>, interp: InterpolationMode) {
      unimplemented!()
    }

    /// Returns the transformations currently applied to the context.
    fn current_transform(&self) -> Affine {
      unimplemented!()
    }
}