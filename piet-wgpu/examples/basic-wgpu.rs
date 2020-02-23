/// This example shows how to capture an image by rendering it to a texture, copying the texture to
/// a buffer, and retrieving it from the buffer. This could be used for "taking a screenshot," with
/// the added benefit that this method doesn't require a window to be created.
use std::fs::File;
use std::mem::size_of;

use piet::{Color, RenderContext};
use piet_wgpu::WgpuRenderContext;

async fn run() {
    env_logger::init();

    let adapter = wgpu::Adapter::request(
        &wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::Default,
        },
        wgpu::BackendBit::PRIMARY,
    )
    .unwrap();

    let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
        extensions: wgpu::Extensions {
            anisotropic_filtering: false,
        },
        limits: wgpu::Limits::default(),
    });

    // Rendered image is 256×256 with 32-bit RGBA color
    let width = 800u32;
    let height = 600u32;

    // The output buffer lets us retrieve the data as an array
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        size: (width * height) as u64 * size_of::<u32>() as u64,
        usage: wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::COPY_DST,
    });

    let texture_extent = wgpu::Extent3d {
        width,
        height,
        depth: 1,
    };

    // The render pipeline renders data into this texture
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        size: texture_extent,
        array_layer_count: 1,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::COPY_SRC,
    });

    let mut render_ctx = WgpuRenderContext::new(&device);

    let now = std::time::Instant::now();

    for _ in 0..100 {
        // draw stuff
        render_ctx.clear(Color::rgb8(58, 165, 181));
        let red_brush = render_ctx.solid_brush(Color::rgb8(255, 0, 0));
        let rect = piet::kurbo::RoundedRect::new(10.0, 10.0, 300.0, 400.0, 70.0);
        render_ctx.fill(rect, &red_brush);

        let green_brush = render_ctx.solid_brush(Color::rgb8(255, 255, 0));
        let rect = piet::kurbo::RoundedRect::new(400.0, 500.0, 700.0, 550.0, 15.0);
        render_ctx.fill(rect, &green_brush);
    }

    // Set the background to be red
    let command_buffer = {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

        render_ctx.finish();
        render_ctx.render(&mut encoder, &texture.create_default_view(), width, height);

        // Copy the data from the texture to the buffer
        encoder.copy_texture_to_buffer(
            wgpu::TextureCopyView {
                texture: &texture,
                mip_level: 0,
                array_layer: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::BufferCopyView {
                buffer: &output_buffer,
                offset: 0,
                row_pitch: size_of::<u32>() as u32 * width,
                image_height: height,
            },
            texture_extent,
        );

        encoder.finish()
    };

    queue.submit(&[command_buffer]);

    // Write the buffer as a PNG
    if let Ok(mapping) = output_buffer.map_read(0u64, (width * height) as u64 * size_of::<u32>() as u64).await {
        let elapsed = now.elapsed();
        println!("Frame took: {:?}", elapsed);
        let mut png_encoder = png::Encoder::new(File::create("output.png").unwrap(), width, height);
        png_encoder.set_depth(png::BitDepth::Eight);
        png_encoder.set_color(png::ColorType::RGBA);
        png_encoder
            .write_header()
            .unwrap()
            .write_image_data(mapping.as_slice())
            .unwrap();
    }

    // The device will be polled when it is dropped but we can also poll it explicitly
    device.poll(true);
}

fn main() {
    futures::executor::block_on(run());
}
