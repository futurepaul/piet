use piet::RenderContext;
use piet_raqote::RaqoteRenderContext;

use piet_test::draw_test_picture;

use std::fs::File;

use raqote::{DrawTarget, PathBuilder, Source, SolidSource, Winding};

const TEXTURE_WIDTH: i32 = 400;
const TEXTURE_HEIGHT: i32 = 200;

const HIDPI: f64 = 2.0;

fn main() {
    let test_picture_number = std::env::args()
        .skip(1)
        .next()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);

    let mut draw_target = DrawTarget::new(TEXTURE_WIDTH, TEXTURE_HEIGHT);

    // TODO: Replace this with clear
    let mut pb = PathBuilder::new();
    pb.rect(0.0, 0.0, TEXTURE_WIDTH as f32, TEXTURE_HEIGHT as f32);
    let path = pb.finish();
    draw_target.fill(&path, &Source::Solid(SolidSource {
        r: 0xFF,
        g: 0xFF,
        b: 0xFF,
        a: 0xFF,
    }), Winding::NonZero);

    let mut raqote_context = RaqoteRenderContext::new(&mut draw_target);
    draw_test_picture(&mut raqote_context, test_picture_number).unwrap();

    draw_target.write_png("temp-raqote.png");
}