//! The Raqote backend for the Piet 2D graphics abstraction.

use raqote::{DrawTarget, Path, PathBuilder, Point, SolidSource, Source, Transform, Winding};

//TODO: raqote should export this type
use sw_composite::Image;

//Euclid's Transform2D is now part of raqote, but we still need Angle
use euclid::Angle;

use kurbo::{Affine, PathEl, Rect, Shape, Vec2};

use font_kit::family_name::FamilyName;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;

use piet::{
    new_error, Error, ErrorKind, FillRule, Font, FontBuilder, Gradient, ImageFormat,
    InterpolationMode, LineCap, LineJoin, RenderContext, RoundInto, StrokeStyle, Text, TextLayout,
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
            text: RaqoteText,
            ctx_stack: vec![CtxState::default()],
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
    font: font_kit::font::Font,
    size: f32,
}

pub struct RaqoteFontBuilder {
    family: String,
    size: f32,
    properties: Properties,
}

pub struct RaqoteTextLayout {
    font: RaqoteFont,
    text: String,
}

pub struct RaqoteTextLayoutBuilder(RaqoteTextLayout);

fn split_rgba(rgba: u32) -> (u8, u8, u8, u8) {
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
        LineJoin::Miter => raqote::LineJoin::Mitre,
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

// Convert a RGBA u32 to a ARBG u32
fn rgba_to_arbg(rgba: u32) -> u32 {
    (rgba << 24) | (rgba >> 8)
}

// Generates a 2D transform for rendering linear gradients in Raqot
// If Raqot is given an identity transform, it will render linear gradients from (0, 0) to (256, 0)
// This function generates a transforms such that the linear gradient will be drawn
// between the provided start and end points.
fn linear_points_to_transform(start: Vec2, end: Vec2) -> Transform {
    let gradient_vector = end - start;
    // Move to start point
    let translate = Transform::create_translation(start.x as f32, start.y as f32);
    // Get length of gradient vector
    let length = gradient_vector.hypot() as f32;
    // Linear grandients in raqot go from (0, 0) to (256, 0), this may change in the future
    // Scaling is multiplication not division (2, not 0.5)
    let scale = Transform::create_scale(length / 256.0, length / 256.0);
    // Get correct angle
    let rotation = Transform::create_rotation(-Angle::radians(gradient_vector.atan2() as f32));

    // TODO: Move `inverse()` to Raqote
    translate
        .pre_mul(&rotation)
        .pre_mul(&scale)
        .inverse()
        .unwrap()
}

//TODO fix scaling
fn transform_image_to_rect(rect: Rect, image: &Image) -> Transform {
    let translate = Transform::create_translation(rect.x0 as f32, rect.y0 as f32);

    let rect_width = rect.width();
    let rect_height = rect.height();

    let scale_width = (rect_width / image.width as f64) as f32;
    let scale_height = (rect_height / image.height as f64) as f32;

    //This number seems plausible but it multiplies with overflow
    println!("possible scale: {:?}, {:?}", scale_width, scale_height);

    let scale = Transform::create_scale(2.0, 2.0);

    // TODO: Move `inverse()` to Raqote
    translate.pre_mul(&scale).inverse().unwrap()
}

// Generates a 2D transform for rendering radial gradients in Raqot
fn radial_points_to_transform(center: Vec2, _origin_offset: Vec2, radius: f32) -> Transform {
    // Max distance is 32768
    let scale = Transform::create_scale(radius / 128.0, radius / 128.0);
    let translate = Transform::create_translation(center.x as f32, center.y as f32);

    // TODO: Move `inverse()` to Raqote
    translate.pre_mul(&scale).inverse().unwrap()
}

fn shape_to_path(shape: impl Shape) -> Path {
    let mut builder = PathBuilder::new();
    for el in shape.to_bez_path(1e-3) {
        match el {
            PathEl::Moveto(p) => {
                builder.move_to(p.x as f32, p.y as f32);
            }
            PathEl::Lineto(p) => {
                builder.line_to(p.x as f32, p.y as f32);
            }
            PathEl::Quadto(p1, p2) => {
                builder.quad_to(p1.x as f32, p1.y as f32, p2.x as f32, p2.y as f32);
            }
            PathEl::Curveto(p1, p2, p3) => {
                builder.cubic_to(
                    p1.x as f32,
                    p1.y as f32,
                    p2.x as f32,
                    p2.y as f32,
                    p3.x as f32,
                    p3.y as f32,
                );
            }
            PathEl::Closepath => builder.close(),
        }
    }
    let path = builder.finish();
    path
}

impl<'a> RenderContext for RaqoteRenderContext<'a> {
    // TODO: Maybe this should be a (f32, f32)?
    type Point = Vec2;
    type Coord = f32;
    type Brush = Source;

    type Text = RaqoteText;
    type TextLayout = RaqoteTextLayout;

    type Image = sw_composite::Image;

    fn status(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn solid_brush(&mut self, rgba: u32) -> Result<Self::Brush, Error> {
        let (r, g, b, a) = split_rgba(rgba);
        Ok(Source::Solid(SolidSource { r, g, b, a }))
    }

    fn gradient(&mut self, gradient: Gradient) -> Result<Self::Brush, Error> {
        match gradient {
            Gradient::Linear(gradient) => Ok(Source::LinearGradient(
                raqote::Gradient {
                    stops: gradient
                        .stops
                        .iter()
                        .map(|stop| raqote::GradientStop {
                            position: stop.pos,
                            color: rgba_to_arbg(stop.rgba),
                        })
                        .collect(),
                },
                linear_points_to_transform(gradient.start, gradient.end),
            )),
            Gradient::Radial(gradient) => Ok(Source::RadialGradient(
                raqote::Gradient {
                    stops: gradient
                        .stops
                        .iter()
                        .map(|stop| raqote::GradientStop {
                            position: stop.pos,
                            color: rgba_to_arbg(stop.rgba),
                        })
                        .collect(),
                },
                radial_points_to_transform(
                    gradient.center,
                    gradient.origin_offset,
                    gradient.radius as f32,
                ),
            )),
        }
    }

    fn clear(&mut self, _rgb: u32) {
        // TODO: Fork Raqote to either (or both)
        // 1. Clear command
        // 2. Expose width and height
    }

    fn stroke(
        &mut self,
        shape: impl Shape,
        brush: &Self::Brush,
        width: impl RoundInto<Self::Coord>,
        style: Option<&StrokeStyle>,
    ) {
        let path = shape_to_path(shape);

        // TODO: Factor this out
        let cap = style
            .and_then(|style| style.line_cap)
            .map(convert_line_cap)
            .unwrap_or(raqote::LineCap::Butt);

        let join = style
            .and_then(|style| style.line_join)
            .map(convert_line_join)
            .unwrap_or(raqote::LineJoin::Mitre);

        let width = width.round_into();

        let mitre_limit = style
            .and_then(|style| style.miter_limit)
            .map(|miter_limit| miter_limit as f32)
            .unwrap_or(10.0);

        let (dash_array, dash_offset) = style
            .and_then(|style| style.dash.as_ref())
            .map(convert_dash)
            .unwrap_or_else(|| (vec![], 0.0));

        let stroke_style = raqote::StrokeStyle {
            cap,
            join,
            width,
            mitre_limit,
            dash_array,
            dash_offset,
        };

        self.draw_target.stroke(&path, brush, &stroke_style);
    }

    fn fill(&mut self, shape: impl Shape, brush: &Self::Brush, fill_rule: FillRule) {
        let path = shape_to_path(shape);

        let winding_mode = match fill_rule {
            FillRule::EvenOdd => Winding::EvenOdd,
            FillRule::NonZero => Winding::NonZero,
        };

        self.draw_target.fill(&path, brush, winding_mode);
    }

    fn clip(&mut self, _shape: impl Shape, _fill_rule: FillRule) {
        // TODO
    }

    fn text(&mut self) -> &mut Self::Text {
        // TODO: Do actual text
        &mut self.text
    }

    fn draw_text(
        &mut self,
        layout: &Self::TextLayout,
        pos: impl RoundInto<Self::Point>,
        brush: &Self::Brush,
    ) {
        let pos = pos.round_into();
        let point = Point::new(pos.x as f32, pos.y as f32);

        self.draw_target.draw_text(
            &layout.font.font,
            layout.font.size,
            &layout.text,
            point,
            brush,
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

        return Ok(Image {
            width: width as i32,
            height: height as i32,
            data: image,
        });
    }

    fn draw_image(
        &mut self,
        image: &Self::Image,
        rect: impl Into<Rect>,
        _interp: InterpolationMode,
    ) {
        let rect = rect.into();

        let transform = transform_image_to_rect(rect, image);

        //TODO: I don't know how to get a non-reference of image other than this dumb thing
        let my_own_image = Image {
            width: image.width,
            height: image.height,
            data: image.data.clone(),
        };

        let path = shape_to_path(rect);

        //QUESTION which winding is appropriate here?
        self.draw_target.fill(
            &path,
            //TODO figure out why scaling is off
            &Source::Image(my_own_image, transform),
            Winding::NonZero,
        );
    }
}

impl Text for RaqoteText {
    type Coord = f32;

    type Font = RaqoteFont;
    type FontBuilder = RaqoteFontBuilder;
    type TextLayout = RaqoteTextLayout;
    type TextLayoutBuilder = RaqoteTextLayoutBuilder;

    fn new_font_by_name(
        &mut self,
        name: &str,
        size: impl RoundInto<Self::Coord>,
    ) -> Result<Self::FontBuilder, Error> {
        Ok(RaqoteFontBuilder {
            family: name.to_owned(),
            size: size.round_into(),
            properties: Properties::new(),
        })
    }

    fn new_text_layout(
        &mut self,
        font: &Self::Font,
        text: &str,
    ) -> Result<Self::TextLayoutBuilder, Error> {
        let text_layout = RaqoteTextLayout {
            font: font.clone(),
            text: text.to_owned(),
        };
        Ok(RaqoteTextLayoutBuilder(text_layout))
    }
}

impl FontBuilder for RaqoteFontBuilder {
    type Out = RaqoteFont;

    fn build(self) -> Result<Self::Out, Error> {
        let font = SystemSource::new()
            .select_best_match(&[FamilyName::SansSerif], &self.properties)
            .unwrap()
            .load()
            .unwrap();

        Ok(RaqoteFont {
            font: font,
            size: self.size,
        })
    }
}

impl Font for RaqoteFont {}

impl TextLayoutBuilder for RaqoteTextLayoutBuilder {
    type Out = RaqoteTextLayout;

    fn build(self) -> Result<Self::Out, Error> {
        Ok(self.0)
    }
}

impl TextLayout for RaqoteTextLayout {
    type Coord = f32;

    fn width(&self) -> Self::Coord {
        20.0
    }
}
