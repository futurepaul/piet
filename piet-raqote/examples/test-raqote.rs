use raqote::{DrawOptions, DrawTarget, PathBuilder, SolidSource, Source};

fn main() {
    let mut draw_target = DrawTarget::new(400, 400);

    let blue = Source::Solid(SolidSource {
        r: 0x0,
        g: 0x0,
        b: 0xFF,
        a: 0xFF,
    });

    let mut pb = PathBuilder::new();
    pb.move_to(10.0, 10.0);
    pb.line_to(10.0, 110.0);
    pb.line_to(110.0, 110.0);
    pb.line_to(110.0, 60.0);

    //Finish without closing
    let path = pb.finish();

    draw_target.fill(&path, &blue, &DrawOptions::new());

    let mut pb = PathBuilder::new();
    pb.move_to(120.0, 10.0);
    pb.line_to(120.0, 110.0);
    pb.line_to(220.0, 110.0);
    pb.line_to(220.0, 60.0);

    //Close, then finish
    pb.close();
    let path = pb.finish();

    draw_target.fill(&path, &blue, &DrawOptions::new());

    draw_target.write_png("test-raqote.png").unwrap();
}
