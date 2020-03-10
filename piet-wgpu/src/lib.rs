mod batch;
mod gradient;

use gradient::{GradientStore, LinearGradient};

use batch::{BatchList, RenderPipelines};

use std::borrow::Cow;

use piet::kurbo::{Affine, PathEl, Point as PietPoint, Rect, Shape};

use piet::{
    Color, Error, FixedGradient, Font, FontBuilder, HitTestPoint, HitTestTextPosition, ImageFormat,
    InterpolationMode, IntoBrush, RenderContext, StrokeStyle, Text, TextLayout, TextLayoutBuilder,
};

use lyon::math::{point, Point};
use lyon::path::Path;
use lyon::tessellation::geometry_builder::*;
use lyon::tessellation::{
    self, FillOptions, FillTessellator, LineCap, LineJoin, StrokeOptions, StrokeTessellator,
};

use zerocopy::AsBytes;

// TODO: Query this from the device.
const MSAA_SAMPLES: u32 = 4;

#[repr(C)]
#[derive(Debug, Clone, Copy, AsBytes)]
pub struct WgpuVertex {
    pos: [f32; 2],
    prim_id: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, AsBytes)]
struct WgpuPrimitive {
    color: [f32; 4],
}

#[derive(Debug)]
struct WgpuVertexCtor {
    prim_id: u32,
}

impl FillVertexConstructor<WgpuVertex> for WgpuVertexCtor {
    fn new_vertex(&mut self, position: Point, _: tessellation::FillAttributes) -> WgpuVertex {
        assert!(!position.x.is_nan());
        assert!(!position.y.is_nan());

        WgpuVertex {
            pos: position.to_array(),
            prim_id: self.prim_id,
        }
    }
}

impl StrokeVertexConstructor<WgpuVertex> for WgpuVertexCtor {
    fn new_vertex(&mut self, position: Point, _: tessellation::StrokeAttributes) -> WgpuVertex {
        assert!(!position.x.is_nan());
        assert!(!position.y.is_nan());

        WgpuVertex {
            pos: position.to_array(),
            prim_id: self.prim_id,
        }
    }
}

struct LyonCtx {
    fill_tess: FillTessellator,
    stroke_tess: StrokeTessellator,
    mesh: VertexBuffers<WgpuVertex, u32>,
    primitives: Vec<WgpuPrimitive>,
}

impl LyonCtx {
    fn new() -> LyonCtx {
        LyonCtx {
            fill_tess: FillTessellator::new(),
            stroke_tess: StrokeTessellator::new(),
            mesh: VertexBuffers::new(),
            primitives: Vec::new(),
        }
    }
}

#[rustfmt::skip]
const IDENTITY_MATRIX: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 1.0, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

fn orthographic_projection(width: f64, height: f64) -> [f32; 16] {
    #[cfg_attr(rustfmt, rustfmt_skip)]
    [
        2.0 / width as f32, 0.0, 0.0, 0.0,
        0.0, 2.0 / height as f32, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        -1.0, -1.0, 0.0, 1.0,
    ]
}

pub struct WgpuCtx<'a> {
    pub device: &'a wgpu::Device,
    pub clear_color: wgpu::Color,
    pub msaa_texture: wgpu::Texture,
    pub current_size: (u32, u32),
    pub transform_buffer: wgpu::Buffer,
    pub current_transform: [f32; 16],
    pub batch_list: BatchList,
    pub render_pipelines: RenderPipelines,
    pub global_bind_group_layout: wgpu::BindGroupLayout,
    pub gradient_store: GradientStore,
}

impl WgpuCtx<'_> {
    fn new(device: &wgpu::Device, width: u32, height: u32) -> WgpuCtx {
        let msaa_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width,
                height,
                depth: 1,
            },
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: MSAA_SAMPLES,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        });

        let transform_buffer = device.create_buffer_with_data(
            IDENTITY_MATRIX.as_bytes(),
            wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        );

        let global_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                bindings: &[wgpu::BindGroupLayoutBinding {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                }],
            });

        let render_pipelines = RenderPipelines::new(device, &global_bind_group_layout);

        WgpuCtx {
            device,
            clear_color: wgpu::Color::WHITE,
            current_transform: IDENTITY_MATRIX,
            transform_buffer,
            current_size: (width, height),
            msaa_texture,
            batch_list: BatchList::new(),
            render_pipelines,
            global_bind_group_layout,
            gradient_store: GradientStore::new(),
        }
    }
}

pub struct WgpuRenderContext<'a> {
    lyon_ctx: LyonCtx,
    wgpu_ctx: WgpuCtx<'a>,
}

impl WgpuRenderContext<'_> {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> WgpuRenderContext {
        WgpuRenderContext {
            lyon_ctx: LyonCtx::new(),
            wgpu_ctx: WgpuCtx::new(device, width, height),
        }
    }

    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        texture: &wgpu::TextureView,
        width: u32,
        height: u32,
    ) {
        // Update the transform buffer
        let ortho_proj = orthographic_projection(width as f64, height as f64);
        if self.wgpu_ctx.current_transform != ortho_proj {
            let temp_buffer = self
                .wgpu_ctx
                .device
                .create_buffer_with_data(ortho_proj.as_bytes(), wgpu::BufferUsage::COPY_SRC);

            encoder.copy_buffer_to_buffer(
                &temp_buffer,
                0,
                &self.wgpu_ctx.transform_buffer,
                0,
                16 * 4,
            );

            self.wgpu_ctx.current_transform = ortho_proj;
        }

        // Create bind group!
        let bind_group = self
            .wgpu_ctx
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.wgpu_ctx.global_bind_group_layout,
                bindings: &[wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &self.wgpu_ctx.transform_buffer,
                        range: 0..(16 * 4),
                    },
                }],
            });

        if self.wgpu_ctx.current_size != (width, height) {
            self.wgpu_ctx.current_size = (width, height);
            self.wgpu_ctx.msaa_texture =
                self.wgpu_ctx
                    .device
                    .create_texture(&wgpu::TextureDescriptor {
                        size: wgpu::Extent3d {
                            width,
                            height,
                            depth: 1,
                        },
                        array_layer_count: 1,
                        mip_level_count: 1,
                        sample_count: MSAA_SAMPLES,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                    });
        }

        let msaa_texture_view = self.wgpu_ctx.msaa_texture.create_default_view();

        self.wgpu_ctx.batch_list.prepare_for_render(
            self.wgpu_ctx.device,
            encoder,
            &self.wgpu_ctx.render_pipelines,
            &mut self.wgpu_ctx.gradient_store,
        );

        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: &msaa_texture_view,
                resolve_target: Some(texture),
                load_op: wgpu::LoadOp::Clear,
                store_op: wgpu::StoreOp::Store,
                clear_color: self.wgpu_ctx.clear_color,
            }],
            depth_stencil_attachment: None,
        });

        // Bind the global bind group
        rpass.set_bind_group(0, &bind_group, &[]);
        self.wgpu_ctx
            .batch_list
            .render(&self.wgpu_ctx.render_pipelines, &mut rpass);
    }
}

pub struct WgpuText;
pub struct WgpuTextLayout;
pub struct WgpuFontBuilder;
pub struct WgpuFont;
pub struct WgpuTextLayoutBuilder;

#[derive(Clone)]
pub enum WgpuBrush {
    Solid(wgpu::Color),
    LinearGradient(LinearGradient),
}

impl WgpuBrush {
    fn is_solid(&self) -> bool {
        match self {
            WgpuBrush::Solid(..) => true,
            _ => false,
        }
    }

    fn is_linear_gradient(&self) -> bool {
        match self {
            WgpuBrush::LinearGradient(..) => true,
            _ => false,
        }
    }
}

impl Font for WgpuFont {}

impl FontBuilder for WgpuFontBuilder {
    type Out = WgpuFont;

    fn build(self) -> Result<Self::Out, Error> {
        unimplemented!()
    }
}

impl TextLayoutBuilder for WgpuTextLayoutBuilder {
    type Out = WgpuTextLayout;

    fn build(self) -> Result<Self::Out, Error> {
        unimplemented!()
    }
}

impl Text for WgpuText {
    type FontBuilder = WgpuFontBuilder;
    type Font = WgpuFont;

    type TextLayoutBuilder = WgpuTextLayoutBuilder;
    type TextLayout = WgpuTextLayout;

    fn new_font_by_name(&mut self, name: &str, size: f64) -> Self::FontBuilder {
        unimplemented!()
    }

    fn new_text_layout(&mut self, font: &Self::Font, text: &str) -> Self::TextLayoutBuilder {
        unimplemented!()
    }
}

impl IntoBrush<WgpuRenderContext<'_>> for WgpuBrush {
    fn make_brush(
        &self,
        _piet: &mut WgpuRenderContext,
        _bbox: impl FnOnce() -> Rect,
    ) -> std::borrow::Cow<WgpuBrush> {
        Cow::Borrowed(self)
    }
}

impl TextLayout for WgpuTextLayout {
    fn width(&self) -> f64 {
        unimplemented!()
    }

    fn hit_test_point(&self, point: PietPoint) -> HitTestPoint {
        unimplemented!()
    }

    fn hit_test_text_position(&self, text_position: usize) -> Option<HitTestTextPosition> {
        unimplemented!()
    }
}

pub fn split_rgba(rgba: &Color) -> (f64, f64, f64, f64) {
    let rgba = rgba.as_rgba_u32();
    (
        (rgba >> 24) as f64 / 255.0,
        ((rgba >> 16) & 255) as f64 / 255.0,
        ((rgba >> 8) & 255) as f64 / 255.0,
        (rgba & 255) as f64 / 255.0,
    )
}

#[derive(Clone, Copy, Debug)]
struct WgpuPoint {
    x: f32,
    y: f32,
}

fn to_point<P: piet::RoundInto<WgpuPoint>>(p: P) -> WgpuPoint {
    p.round_into()
}

impl piet::RoundFrom<PietPoint> for WgpuPoint {
    fn round_from(point: PietPoint) -> WgpuPoint {
        WgpuPoint {
            x: point.x as f32,
            y: point.y as f32,
        }
    }
}

fn shape_to_path(shape: impl Shape) -> Path {
    let mut builder = Path::builder();
    for el in shape.to_bez_path(1e-3) {
        match el {
            PathEl::MoveTo(p) => {
                let p = to_point(p);
                builder.move_to(point(p.x, p.y));
            }
            PathEl::LineTo(p) => {
                let p = to_point(p);
                builder.line_to(point(p.x, p.y));
            }
            PathEl::QuadTo(p1, p2) => {
                let p1 = to_point(p1);
                let p2 = to_point(p2);
                builder.quadratic_bezier_to(point(p1.x, p1.y), point(p2.x, p2.y));
            }
            PathEl::CurveTo(p1, p2, p3) => {
                let p1 = to_point(p1);
                let p2 = to_point(p2);
                let p3 = to_point(p3);
                builder.cubic_bezier_to(point(p1.x, p1.y), point(p2.x, p2.y), point(p3.x, p3.y));
            }
            PathEl::ClosePath => builder.close(),
        }
    }

    builder.build()
}

fn convert_stroke_style(style: &StrokeStyle, width: f32, tolerance: f32) -> StrokeOptions {
    let line_join = match style.line_join.unwrap_or(piet::LineJoin::Miter) {
        piet::LineJoin::Miter => LineJoin::Miter,
        piet::LineJoin::Round => LineJoin::Round,
        piet::LineJoin::Bevel => LineJoin::Bevel,
    };

    let line_cap = match style.line_cap.unwrap_or(piet::LineCap::Butt) {
        piet::LineCap::Butt => LineCap::Butt,
        piet::LineCap::Round => LineCap::Round,
        piet::LineCap::Square => LineCap::Square,
    };

    let miter_limit = style.miter_limit.unwrap_or(10.0);

    StrokeOptions::tolerance(tolerance)
        .with_line_join(line_join)
        .with_line_cap(line_cap)
        .with_miter_limit(miter_limit as f32)
        .with_line_width(width)
}

impl RenderContext for WgpuRenderContext<'_> {
    /// The type of a "brush".
    ///
    /// Represents solid colors and gradients.
    type Brush = WgpuBrush;

    /// An associated factory for creating text layouts and related resources.
    type Text = WgpuText;
    type TextLayout = WgpuTextLayout;

    /// The associated type of an image.
    type Image = ();

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
        let (r, g, b, a) = split_rgba(&color);
        WgpuBrush::Solid(wgpu::Color { r, g, b, a })
    }

    /// Create a new gradient brush.
    fn gradient(&mut self, gradient: impl Into<FixedGradient>) -> Result<Self::Brush, Error> {
        let gradient: FixedGradient = gradient.into();
        let linear_gradient = match gradient {
            FixedGradient::Linear(linear) => linear,
            _ => unimplemented!(),
        };
        Ok(WgpuBrush::LinearGradient(LinearGradient::from(
            linear_gradient,
        )))
    }

    /// Clear the canvas with the given color.
    ///
    /// Note: only opaque colors are meaningful.
    fn clear(&mut self, color: Color) {
        let (r, g, b, a) = split_rgba(&color);
        self.wgpu_ctx.clear_color = wgpu::Color { r, g, b, a };
        self.lyon_ctx = LyonCtx::new();
    }

    /// Stroke a shape.
    fn stroke(&mut self, shape: impl Shape, brush: &impl IntoBrush<Self>, width: f64) {
        self.stroke_styled(shape, brush, width, &StrokeStyle::new());
    }

    /// Stroke a shape, with styled strokes.
    fn stroke_styled(
        &mut self,
        shape: impl Shape,
        brush: &impl IntoBrush<Self>,
        width: f64,
        style: &StrokeStyle,
    ) {
        let path = shape_to_path(&shape);
        let brush = brush.make_brush(self, || shape.bounding_box());
        let stroke_opts = convert_stroke_style(style, width as f32, 0.01);

        let (mesh_buffer, prim_id) = self.wgpu_ctx.batch_list.request_mesh(
            &self.wgpu_ctx.device,
            &self.wgpu_ctx.render_pipelines,
            &brush,
        );

        self.lyon_ctx.stroke_tess.tessellate(
            &path,
            &stroke_opts,
            &mut BuffersBuilder::new(mesh_buffer, WgpuVertexCtor { prim_id: prim_id }),
        );
    }

    /// Fill a shape, using non-zero fill rule.
    fn fill(&mut self, shape: impl Shape, brush: &impl IntoBrush<Self>) {
        let path = shape_to_path(&shape);
        let brush = brush.make_brush(self, || shape.bounding_box());
        let (mesh_buffer, prim_id) = self.wgpu_ctx.batch_list.request_mesh(
            &self.wgpu_ctx.device,
            &self.wgpu_ctx.render_pipelines,
            &brush,
        );

        // Tesselate and adds to mesh
        self.lyon_ctx.fill_tess.tessellate(
            &path,
            &FillOptions::tolerance(0.01),
            &mut BuffersBuilder::new(mesh_buffer, WgpuVertexCtor { prim_id }),
        );
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
        pos: impl Into<PietPoint>,
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
        // Do nothing for now...
        Ok(())
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
    fn draw_image(
        &mut self,
        image: &Self::Image,
        rect: impl Into<Rect>,
        interp: InterpolationMode,
    ) {
        unimplemented!()
    }

    /// Returns the transformations currently applied to the context.
    fn current_transform(&self) -> Affine {
        unimplemented!()
    }
}
