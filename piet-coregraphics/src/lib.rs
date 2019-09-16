//! The CoreGraphics backend for the Piet 2D graphics abstraction.

use std::borrow::Cow;

use core_graphics::base::CGFloat;
use core_graphics::context::{CGContext, CGLineCap, CGLineJoin};
use core_graphics::image::CGImage;

use piet::kurbo::{Affine, PathEl, Point, QuadBez, Rect, Shape};

use piet::{
    new_error, Color, Error, ErrorKind, FixedGradient, Font, FontBuilder, ImageFormat,
    InterpolationMode, IntoBrush, LineCap, LineJoin, RenderContext, RoundInto, StrokeStyle, Text,
    TextLayout, TextLayoutBuilder,
};

pub struct CoreGraphicsContext<'a> {
    // Cairo has this as Clone and with &self methods, but we do this to avoid
    // concurrency problems.
    ctx: &'a mut CGContext,
    text: CoreGraphicsText,
}

impl<'a> CoreGraphicsContext<'a> {
    pub fn new(ctx: &mut CGContext) -> CoreGraphicsContext {
        CoreGraphicsContext {
            ctx,
            text: CoreGraphicsText,
        }
    }
}

#[derive(Clone)]
pub enum Brush {
    Solid(u32),
}

pub struct CoreGraphicsFont;
impl Font for CoreGraphicsFont {}

pub struct CoreGraphicsFontBuilder;

impl FontBuilder for CoreGraphicsFontBuilder {
    type Out = CoreGraphicsFont;

    fn build(self) -> Result<Self::Out, Error> {
        panic!()
    }
}
pub struct CoreGraphicsLayout;

impl TextLayout for CoreGraphicsLayout {
    fn width(&self) -> f64 {
        panic!();
    }
}

pub struct CoreGraphicsTextLayoutBuilder {}

impl TextLayoutBuilder for CoreGraphicsTextLayoutBuilder {
    type Out = CoreGraphicsLayout;

    fn build(self) -> Result<Self::Out, Error> {
        panic!()
    }
}

pub struct CoreGraphicsText;

impl Text for CoreGraphicsText {
    type FontBuilder = CoreGraphicsFontBuilder;
    type Font = CoreGraphicsFont;
    type TextLayoutBuilder = CoreGraphicsTextLayoutBuilder;
    type TextLayout = CoreGraphicsLayout;

    fn new_font_by_name(&mut self, name: &str, size: f64) -> Self::FontBuilder {
        panic!()
    }

    fn new_text_layout(&mut self, font: &Self::Font, text: &str) -> Self::TextLayoutBuilder {
        panic!()
    }
}

// TODO: This cannot be used yet because the `piet::RenderContext` trait
// needs to expose a way to create stroke styles.
/*pub struct StrokeStyle {
    line_join: Option<CGLineJoin>,
    line_cap: Option<CGLineCap>,
    dash: Option<(Vec<f64>, f64)>,
    miter_limit: Option<f64>,
}

impl StrokeStyle {
    pub fn new() -> StrokeStyle {
        StrokeStyle {
            line_join: None,
            line_cap: None,
            dash: None,
            miter_limit: None,
        }
    }

    pub fn line_join(mut self, line_join: CGLineJoin) -> Self {
        self.line_join = Some(line_join);
        self
    }

    pub fn line_cap(mut self, line_cap: CGLineCap) -> Self {
        self.line_cap = Some(line_cap);
        self
    }

    pub fn dash(mut self, dashes: Vec<f64>, offset: f64) -> Self {
        self.dash = Some((dashes, offset));
        self
    }

    pub fn miter_limit(mut self, miter_limit: f64) -> Self {
        self.miter_limit = Some(miter_limit);
        self
    }
}*/

impl<'a> RenderContext for CoreGraphicsContext<'a> {
    type Brush = Brush;
    type Text = CoreGraphicsText;
    type TextLayout = CoreGraphicsLayout;
    type Image = CGImage;
    //type StrokeStyle = StrokeStyle;

    fn clear(&mut self, color: Color) {
        let rgba = color.as_rgba_u32();
        self.ctx.set_rgb_fill_color(
            byte_to_frac(rgba >> 24),
            byte_to_frac(rgba >> 16),
            byte_to_frac(rgba >> 8),
            1.0,
        );
        self.ctx.fill_rect(self.ctx.clip_bounding_box());
    }

    fn solid_brush(&mut self, color: Color) -> Brush {
        Brush::Solid(color.as_rgba_u32())
    }

    fn gradient(&mut self, gradient: impl Into<FixedGradient>) -> Result<Brush, Error> {
        unimplemented!()
    }

    /// Fill a shape.
    fn fill(&mut self, shape: impl Shape, brush: &impl IntoBrush<Self>) {
        let brush = brush.make_brush(self, || shape.bounding_box());
        self.set_path(shape);
        self.set_brush(&*brush);
        self.ctx.fill_path();
    }

    fn fill_even_odd(&mut self, shape: impl Shape, brush: &impl IntoBrush<Self>) {
        let brush = brush.make_brush(self, || shape.bounding_box());
        self.set_path(shape);
        self.set_brush(&*brush);
        self.ctx.eo_fill_path();
    }

    fn clip(&mut self, shape: impl Shape) {
        self.set_path(shape);
        self.ctx.clip();
    }

    fn stroke(&mut self, shape: impl Shape, brush: &impl IntoBrush<Self>, width: f64) {
        let brush = brush.make_brush(self, || shape.bounding_box());
        self.set_path(shape);
        self.set_stroke(width, None);
        self.set_brush(&*brush);
        self.ctx.stroke_path();
    }

    fn stroke_styled(
        &mut self,
        shape: impl Shape,
        brush: &impl IntoBrush<Self>,
        width: f64,
        style: &StrokeStyle,
    ) {
        unimplemented!();
    }

    fn text(&mut self) -> &mut Self::Text {
        &mut self.text
    }

    fn draw_text(
        &mut self,
        layout: &Self::TextLayout,
        pos: impl Into<Point>,
        brush: &impl IntoBrush<Self>,
    ) {
        unimplemented!()
    }

    fn save(&mut self) -> Result<(), Error> {
        self.ctx.save();
        Ok(())
    }

    fn restore(&mut self) -> Result<(), Error> {
        self.ctx.restore();
        Ok(())
    }

    fn finish(&mut self) -> Result<(), Error> {
        unimplemented!()
    }

    fn transform(&mut self, transform: Affine) {
        unimplemented!()
    }

    fn make_image(
        &mut self,
        width: usize,
        height: usize,
        buf: &[u8],
        format: ImageFormat,
    ) -> Result<Self::Image, Error> {
        unimplemented!()
    }

    fn draw_image(
        &mut self,
        image: &Self::Image,
        rect: impl Into<Rect>,
        interp: InterpolationMode,
    ) {
        unimplemented!()
    }

    fn status(&mut self) -> Result<(), Error> {
        unimplemented!()
    }
}

// TODO copy / pasted this from cairo
impl<'a> IntoBrush<CoreGraphicsContext<'a>> for Brush {
    fn make_brush<'b>(
        &'b self,
        _piet: &mut CoreGraphicsContext,
        _bbox: impl FnOnce() -> Rect,
    ) -> std::borrow::Cow<'b, Brush> {
        Cow::Borrowed(self)
    }
}

fn convert_line_join(line_join: LineJoin) -> CGLineJoin {
    match line_join {
        LineJoin::Miter => CGLineJoin::CGLineJoinMiter,
        LineJoin::Round => CGLineJoin::CGLineJoinRound,
        LineJoin::Bevel => CGLineJoin::CGLineJoinBevel,
    }
}

fn convert_line_cap(line_cap: LineCap) -> CGLineCap {
    match line_cap {
        LineCap::Butt => CGLineCap::CGLineCapButt,
        LineCap::Round => CGLineCap::CGLineCapRound,
        LineCap::Square => CGLineCap::CGLineCapSquare,
    }
}

impl<'a> CoreGraphicsContext<'a> {
    /// Set the source pattern to the brush.
    ///
    /// Cairo is super stateful, and we're trying to have more retained stuff.
    /// This is part of the impedance matching.
    fn set_fill_brush(&mut self, brush: &Brush) {
        match *brush {
            Brush::Solid(rgba) => self.ctx.set_rgb_fill_color(
                byte_to_frac(rgba >> 24),
                byte_to_frac(rgba >> 16),
                byte_to_frac(rgba >> 8),
                byte_to_frac(rgba),
            ),
        }
    }

    fn set_brush(&mut self, brush: &Brush) {
        match *brush {
            Brush::Solid(rgba) => self.ctx.set_rgb_stroke_color(
                byte_to_frac(rgba >> 24),
                byte_to_frac(rgba >> 16),
                byte_to_frac(rgba >> 8),
                byte_to_frac(rgba),
            ),
        }
    }

    /// Set the stroke parameters.
    fn set_stroke(&mut self, width: f64, style: Option<&StrokeStyle>) {
        self.ctx.set_line_width(width);

        let line_join = style
            .and_then(|style| style.line_join)
            .unwrap_or(LineJoin::Miter);
        self.ctx.set_line_join(convert_line_join(line_join));

        let line_cap = style
            .and_then(|style| style.line_cap)
            .unwrap_or(LineCap::Butt);
        self.ctx.set_line_cap(convert_line_cap(line_cap));

        let miter_limit = style.and_then(|style| style.miter_limit).unwrap_or(10.0);
        self.ctx.set_miter_limit(miter_limit);

        match style.and_then(|style| style.dash.as_ref()) {
            None => self.ctx.set_line_dash(0.0, &[]),
            Some((dashes, offset)) => self.ctx.set_line_dash(*offset, dashes),
        }
    }

    fn set_path(&mut self, shape: impl Shape) {
        // This shouldn't be necessary, we always leave the context in no-path
        // state. But just in case, and it should be harmless.
        self.ctx.begin_path();
        let mut last = Point::ZERO;
        for el in shape.to_bez_path(1e-3) {
            match el {
                PathEl::MoveTo(p) => {
                    self.ctx.move_to_point(p.x, p.y);
                    last = p;
                }
                PathEl::LineTo(p) => {
                    self.ctx.add_line_to_point(p.x, p.y);
                    last = p;
                }
                PathEl::QuadTo(p1, p2) => {
                    let q = QuadBez::new(last, p1, p2);
                    let c = q.raise();
                    self.ctx
                        .add_curve_to_point(c.p1.x, c.p1.y, c.p2.x, c.p2.y, p2.x, p2.y);
                    last = p2;
                }
                PathEl::CurveTo(p1, p2, p3) => {
                    self.ctx
                        .add_curve_to_point(p1.x, p1.y, p2.x, p2.y, p3.x, p3.y);
                    last = p3;
                }
                PathEl::ClosePath => self.ctx.close_path(),
            }
        }
    }
}

fn byte_to_frac(byte: u32) -> f64 {
    ((byte & 255) as f64) * (1.0 / 255.0)
}
