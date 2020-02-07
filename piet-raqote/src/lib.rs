//! The Raqote backend for the Piet 2D graphics abstraction.

// TODO: dpi scaling!!

use std::borrow::Cow;

use raqote::{
    Color as RaqoteColor, DrawOptions, DrawTarget, ExtendMode, FilterMode, Path, PathBuilder,
    Point, SolidSource, Source, Spread, Transform, Winding,
};

use piet::kurbo::{Affine, PathEl, Point as PietPoint, Rect, Shape, Vec2};

use font_kit::family_name::FamilyName;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;

use skribo::{make_layout, FontRef, Layout, TextStyle};

use piet::{
    new_error, Color, Error, ErrorKind, FixedGradient, Font, FontBuilder, GradientStop,
    HitTestPoint, HitTestTextPosition, ImageFormat, InterpolationMode, IntoBrush, LineCap,
    LineJoin, RenderContext, RoundFrom, RoundInto, StrokeStyle, Text, TextLayout,
    TextLayoutBuilder,
};

#[derive(Default)]
struct CtxState {
    transform: Affine,
}

pub struct RaqoteRenderContext<'a> {
    draw_target: &'a mut DrawTarget,
    ctx_stack: Vec<CtxState>,

    // TODO: Do actual text
    text: RaqoteText,
}

impl<'a> RaqoteRenderContext<'a> {
    pub fn new(draw_target: &'a mut DrawTarget) -> RaqoteRenderContext<'a> {
        RaqoteRenderContext {
            draw_target,
            ctx_stack: vec![CtxState::default()],
            text: RaqoteText,
        }
    }

    fn current_transform(&self) -> Affine {
        // This is an unwrap because we protect the invariant.
        self.ctx_stack.last().unwrap().transform
    }

    fn pop_state(&mut self) {
        self.ctx_stack.pop();
    }
}

pub struct RaqoteText;

#[derive(Clone)]
pub struct RaqoteFont {
    font: FontRef,
    size: f32,
}

pub struct RaqoteFontBuilder {
    family: String,
    size: f32,
    properties: Properties,
}

pub struct RaqoteTextLayout {
    // TODO: Store reference?
    font: RaqoteFont,
    layout: Layout,
}

pub struct RaqoteTextLayoutBuilder {
    // TODO: Store reference?
    font: RaqoteFont,
    text: String,
}

//We need this struct to avoid lifetime issues with raqote's Image type
pub struct InternalImage {
    width: usize,
    height: usize,
    data: Vec<u32>,
}

pub struct RaqotePoint(pub Point);

fn split_rgba(rgba: Color) -> (u8, u8, u8, u8) {
    let rgba = rgba.as_rgba_u32();
    (
        (rgba >> 24) as u8,
        ((rgba >> 16) & 255) as u8,
        ((rgba >> 8) & 255) as u8,
        (rgba & 255) as u8,
    )
}

fn convert_line_join(line_join: LineJoin) -> raqote::LineJoin {
    match line_join {
        LineJoin::Round => raqote::LineJoin::Round,
        LineJoin::Miter => raqote::LineJoin::Miter,
        LineJoin::Bevel => raqote::LineJoin::Bevel,
    }
}

fn convert_line_cap(line_cap: LineCap) -> raqote::LineCap {
    match line_cap {
        LineCap::Butt => raqote::LineCap::Butt,
        LineCap::Round => raqote::LineCap::Round,
        LineCap::Square => raqote::LineCap::Square,
    }
}

fn convert_dash(dash: &(Vec<f64>, f64)) -> (Vec<f32>, f32) {
    // TODO: find cheaper way to do this?
    (dash.0.iter().map(|d| *d as f32).collect(), dash.1 as f32)
}

fn affine_to_transform(affine: Affine) -> Transform {
    let a = affine.as_coeffs();
    Transform::row_major(
        a[0] as f32,
        a[1] as f32,
        a[2] as f32,
        a[3] as f32,
        a[4] as f32,
        a[5] as f32,
    )
}

fn piet_color_to_raqote(color: Color) -> RaqoteColor {
    let c = split_rgba(color);
    RaqoteColor::new(c.3, c.0, c.1, c.2)
}

fn transform_image_to_rect(rect: Rect, image: &raqote::Image) -> Transform {
    let translate = Transform::create_translation(-rect.x0 as f32, -rect.y0 as f32);

    dbg!(rect.width());
    dbg!(image.width);
    // let rect_width = rect.width();
    // let rect_height = rect.height();
    // 1 - (16 * 40 / 1000)
    let scale_width = (1. - (image.width as f64 * rect.width() / 1000.)) as f32;
    let scale_height = (1. - (image.height as f64 * rect.height() / 1000.)) as f32;

    // let scale_width = (image.width as f64 / rect_width) as f32;
    // let scale_height = (image.height as f64 / rect_height) as f32;

    //This number seems plausible but it multiplies with overflow
    println!("possible scale: {:?}, {:?}", scale_width, scale_height);

    let scale = Transform::create_scale(scale_width, scale_height);

    dbg!(scale_width * image.width as f32);

    // TODO: Move `inverse()` to Raqote
    // It used to be: translate.post_mul(&scale)
    translate.post_transform(&scale)
}

fn shape_to_path(shape: impl Shape) -> Path {
    let mut builder = PathBuilder::new();
    for el in shape.to_bez_path(1e-3) {
        match el {
            PathEl::MoveTo(p) => {
                let p = to_point(p);
                builder.move_to(p.x, p.y);
            }
            PathEl::LineTo(p) => {
                let p = to_point(p);
                builder.line_to(p.x, p.y);
            }
            PathEl::QuadTo(p1, p2) => {
                let p1 = to_point(p1);
                let p2 = to_point(p2);
                builder.quad_to(p1.x, p1.y, p2.x, p2.y);
            }
            PathEl::CurveTo(p1, p2, p3) => {
                let p1 = to_point(p1);
                let p2 = to_point(p2);
                let p3 = to_point(p3);
                builder.cubic_to(p1.x, p1.y, p2.x, p2.y, p3.x, p3.y);
            }
            PathEl::ClosePath => builder.close(),
        }
    }

    // TODO: for fills we need to close the path if it's not areadly closed
    let path = builder.finish();
    path
}

fn convert_gradient_stops(stops: Vec<GradientStop>) -> Vec<raqote::GradientStop> {
    stops
        .iter()
        .map(|stop| raqote::GradientStop {
            position: stop.pos,
            color: piet_color_to_raqote(stop.color.clone()),
        })
        .collect()
}

#[derive(Clone)]
pub struct RaqoteBrush<'a>(Source<'a>);

impl<'a> IntoBrush<RaqoteRenderContext<'a>> for RaqoteBrush<'a> {
    fn make_brush<'b>(
        &'b self,
        _piet: &mut RaqoteRenderContext,
        _bbox: impl FnOnce() -> Rect,
    ) -> std::borrow::Cow<'b, RaqoteBrush<'a>> {
        Cow::Borrowed(self)
    }
}

impl<'a> RenderContext for RaqoteRenderContext<'a> {
    // The render context must outlive the brush
    type Brush = RaqoteBrush<'a>;

    //QUESTION Text should of type TextLayout??
    // type Text: Text<TextLayout = Self::TextLayout>;
    type Text = RaqoteText;
    type TextLayout = RaqoteTextLayout;

    //This needs to live as long as source
    type Image = InternalImage;

    fn status(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn solid_brush(&mut self, rgba: Color) -> Self::Brush {
        let (r, g, b, a) = split_rgba(rgba);
        RaqoteBrush(Source::Solid(SolidSource { r, g, b, a }))
    }

    fn current_transform(&self) -> Affine {
        self.current_transform()
    }

    fn gradient(&mut self, gradient: impl Into<FixedGradient>) -> Result<Self::Brush, Error> {
        match gradient.into() {
            FixedGradient::Linear(gradient) => {
                let stops = convert_gradient_stops(gradient.stops);
                let start = to_point((gradient.start.x, gradient.start.y));
                let end = to_point((gradient.end.x, gradient.end.y));

                Ok(RaqoteBrush(Source::new_linear_gradient(
                    raqote::Gradient { stops },
                    start,
                    end,
                    Spread::Pad,
                )))
            }
            FixedGradient::Radial(gradient) => {
                let stops = convert_gradient_stops(gradient.stops);
                let center = to_point((gradient.center.x, gradient.center.y));

                Ok(RaqoteBrush(Source::new_radial_gradient(
                    raqote::Gradient { stops },
                    center,
                    gradient.radius as f32,
                    Spread::Pad,
                )))
            }
        }
    }

    fn clear(&mut self, rgba: Color) {
        // let rgba = (rgb << 8) | 0xff;
        let (r, g, b, a) = split_rgba(rgba);
        let source = SolidSource { r, g, b, a };
        self.draw_target.clear(source);
    }

    fn stroke(&mut self, shape: impl Shape, brush: &impl IntoBrush<Self>, width: f64) {
        let brush = brush.make_brush(self, || shape.bounding_box());
        let path = shape_to_path(shape);

        let mut stroke_style = raqote::StrokeStyle::default();

        stroke_style.width = width.round_into();

        self.draw_target
            .stroke(&path, &brush.0, &stroke_style, &DrawOptions::default());
    }

    fn stroke_styled(
        &mut self,
        shape: impl Shape,
        brush: &impl IntoBrush<Self>,
        width: f64,
        style: &StrokeStyle,
    ) {
        let brush = brush.make_brush(self, || shape.bounding_box());
        let path = shape_to_path(shape);

        // TODO: Factor this out
        let cap = style
            .line_cap
            .map(convert_line_cap)
            .unwrap_or(raqote::LineCap::Butt);

        let join = style
            .line_join
            .map(convert_line_join)
            .unwrap_or(raqote::LineJoin::Miter);

        let width = width.round_into();

        let miter_limit = style
            .miter_limit
            .map(|miter_limit| miter_limit as f32)
            .unwrap_or(10.0);

        let (dash_array, dash_offset) = style
            .dash
            .as_ref()
            .map(convert_dash)
            .unwrap_or_else(|| (vec![], 0.0));

        let stroke_style = raqote::StrokeStyle {
            cap,
            join,
            width,
            miter_limit,
            dash_array,
            dash_offset,
        };

        self.draw_target
            .stroke(&path, &brush.0, &stroke_style, &DrawOptions::default());
    }

    fn fill(&mut self, shape: impl Shape, brush: &impl IntoBrush<Self>) {
        let brush = brush.make_brush(self, || shape.bounding_box());
        let mut path = shape_to_path(shape);

        path.winding = Winding::NonZero;

        //TODO: &brush.0 doesn't feel right but it's the only thing
        //I can get working
        self.draw_target
            .fill(&path, &brush.0, &DrawOptions::default());
    }

    fn fill_even_odd(&mut self, shape: impl Shape, brush: &impl IntoBrush<Self>) {
        let brush = brush.make_brush(self, || shape.bounding_box());
        let mut path = shape_to_path(shape);

        path.winding = Winding::EvenOdd;

        self.draw_target
            .fill(&path, &brush.0, &DrawOptions::default());
    }

    fn clip(&mut self, shape: impl Shape) {
        let mut path = shape_to_path(shape);

        path.winding = Winding::NonZero;
        //QUESTION we don't ever pop clip I hope that's okay?
        self.draw_target.push_clip(&path);
    }

    fn text(&mut self) -> &mut Self::Text {
        // TODO do text better
        &mut self.text
    }

    //TODO why are the glpyhs rotated the wrong way?
    fn draw_text(
        &mut self,
        layout: &Self::TextLayout,
        pos: impl Into<PietPoint>,
        brush: &impl IntoBrush<Self>,
    ) {
        let brush = brush.make_brush(self, || Rect::ZERO);

        let pos = to_point(pos.into());

        let positions = layout
            .layout
            .glyphs
            .iter()
            .map(|glyph| to_point((glyph.offset.x + pos.x, glyph.offset.y + pos.y)))
            .collect::<Vec<Point>>();

        let glyphs = layout
            .layout
            .glyphs
            .iter()
            .map(|glyph| glyph.glyph_id)
            .collect::<Vec<u32>>();

        self.draw_target.draw_glyphs(
            &layout.font.font.font,
            layout.font.size,
            &glyphs,
            &positions,
            &brush.0,
            &DrawOptions::default(),
        );
    }

    fn save(&mut self) -> Result<(), Error> {
        let new_state = CtxState {
            transform: self.current_transform(),
        };
        self.ctx_stack.push(new_state);
        Ok(())
    }

    fn restore(&mut self) -> Result<(), Error> {
        if self.ctx_stack.len() <= 1 {
            return Err(new_error(ErrorKind::StackUnbalance));
        }
        self.pop_state();
        // Move this code into impl to avoid duplication with transform?
        self.draw_target
            .set_transform(&affine_to_transform(self.current_transform()));
        Ok(())
    }

    fn finish(&mut self) -> Result<(), Error> {
        if self.ctx_stack.len() != 1 {
            return Err(new_error(ErrorKind::StackUnbalance));
        }
        self.pop_state();
        Ok(())
    }

    fn transform(&mut self, transform: Affine) {
        self.ctx_stack.last_mut().unwrap().transform *= transform;
        self.draw_target
            .set_transform(&affine_to_transform(self.current_transform()));
    }

    fn make_image(
        &mut self,
        width: usize,
        height: usize,
        buf: &[u8],
        format: ImageFormat,
    ) -> Result<Self::Image, Error> {
        let mut image: Vec<u32> = Vec::new();

        match format {
            ImageFormat::Rgb => {
                for i in buf.chunks(3) {
                    image.push(
                        0xff << 24 | ((i[0] as u32) << 16) | ((i[1] as u32) << 8) | (i[2] as u32),
                    );
                }
            }
            ImageFormat::RgbaPremul => {
                for i in buf.chunks(4) {
                    image.push(
                        ((i[3] as u32) << 24)
                            | ((i[0] as u32) << 16)
                            | ((i[1] as u32) << 8)
                            | (i[2] as u32),
                    )
                }
            }
            ImageFormat::RgbaSeparate => {
                fn premul(x: u8, a: u8) -> u32 {
                    let y = (x as u16) * (a as u16);
                    ((y + (y >> 8) + 0x80) >> 8) as u32
                }
                for i in buf.chunks(4) {
                    let a = i[3];
                    image.push(
                        ((a as u32) << 24)
                            | (premul(i[0], a) << 16)
                            | (premul(i[1], a) << 8)
                            | premul(i[2], a),
                    )
                }
            }
            _ => return Err(new_error(ErrorKind::NotSupported)),
        };

        Ok(InternalImage {
            width: width as usize,
            height: height as usize,
            data: image,
        })
    }

    fn draw_image(
        &mut self,
        image: &Self::Image,
        rect: impl Into<Rect>,
        _interp: InterpolationMode,
    ) {
        let rect = rect.into();

        let raqote_image = raqote::Image {
            width: image.width as i32,
            height: image.height as i32,
            data: &image.data[..],
        };

        let transform = transform_image_to_rect(rect, &raqote_image);
        // let transform = Transform::create_translation(-rect.x0 as f32, -rect.y0 as f32);

        let path = shape_to_path(rect);

        self.draw_target.fill(
            &path,
            //TODO figure out why scaling is off
            &Source::Image(
                raqote_image,
                ExtendMode::Repeat,
                FilterMode::Bilinear,
                transform,
            ),
            &DrawOptions::default(),
        );
        // self.draw_target.draw_image_at(rect.x0 as f32, rect.y0 as f32, &raqote_image, &DrawOptions::default());
    }
}

pub fn to_point<P: RoundInto<RaqotePoint>>(p: P) -> Point {
    p.round_into().0
}

impl From<Point> for RaqotePoint {
    fn from(vec: Point) -> RaqotePoint {
        RaqotePoint(vec.into())
    }
}

impl RoundFrom<(f64, f64)> for RaqotePoint {
    fn round_from(vec: (f64, f64)) -> RaqotePoint {
        RaqotePoint(Point::new(vec.0 as f32, vec.1 as f32))
    }
}

impl RoundFrom<(f32, f32)> for RaqotePoint {
    fn round_from(vec: (f32, f32)) -> RaqotePoint {
        RaqotePoint(Point::new(vec.0 as f32, vec.1 as f32))
    }
}

impl RoundFrom<piet::kurbo::Point> for RaqotePoint {
    fn round_from(point: piet::kurbo::Point) -> RaqotePoint {
        RaqotePoint(Point::new(point.x as f32, point.y as f32))
    }
}

impl RoundFrom<Vec2> for RaqotePoint {
    fn round_from(vec: Vec2) -> RaqotePoint {
        RaqotePoint(Point::new(vec.x as f32, vec.y as f32))
    }
}

impl From<RaqotePoint> for Vec2 {
    fn from(vec: RaqotePoint) -> Vec2 {
        Vec2::new(vec.0.x as f64, vec.0.y as f64)
    }
}

impl Text for RaqoteText {
    type Font = RaqoteFont;
    type FontBuilder = RaqoteFontBuilder;
    type TextLayout = RaqoteTextLayout;
    type TextLayoutBuilder = RaqoteTextLayoutBuilder;

    fn new_font_by_name(&mut self, name: &str, size: f64) -> Self::FontBuilder {
        RaqoteFontBuilder {
            family: name.to_owned(),
            size: size.round_into(),
            properties: Properties::new(),
        }
    }

    fn new_text_layout(&mut self, font: &Self::Font, text: &str) -> Self::TextLayoutBuilder {
        RaqoteTextLayoutBuilder {
            font: font.clone(),
            // TODO: Store a reference?
            text: text.to_owned(),
        }
    }
}

impl FontBuilder for RaqoteFontBuilder {
    type Out = RaqoteFont;

    fn build(self) -> Result<Self::Out, Error> {
        let font = SystemSource::new()
            .select_best_match(
                &[FamilyName::Title(self.family), FamilyName::SansSerif],
                &self.properties,
            )
            .unwrap()
            .load()
            .unwrap();

        Ok(RaqoteFont {
            font: FontRef::new(font),
            size: self.size,
        })
    }
}

impl Font for RaqoteFont {}

//TODO seems like we should be doing some work here?
impl TextLayoutBuilder for RaqoteTextLayoutBuilder {
    type Out = RaqoteTextLayout;

    fn build(self) -> Result<Self::Out, Error> {
        let layout = make_layout(
            &TextStyle {
                size: self.font.size,
            },
            &self.font.font,
            &self.text,
        );

        let text_layout = RaqoteTextLayout {
            font: self.font.clone(),
            layout,
        };

        Ok(text_layout)
    }
}

impl TextLayout for RaqoteTextLayout {
    fn width(&self) -> f64 {
        self.layout.advance.x.into()
    }

    fn hit_test_point(&self, _point: PietPoint) -> HitTestPoint {
        HitTestPoint::default()
    }

    fn hit_test_text_position(&self, _text_position: usize) -> Option<HitTestTextPosition> {
        None
    }
}
