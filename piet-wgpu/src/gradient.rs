use crate::split_rgba;
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
};

#[derive(Debug, Clone, PartialEq)]
pub struct LinearGradient {
    pub handle: GradientHandle,
    pub start: piet::kurbo::Point,
    pub end: piet::kurbo::Point,
}

impl From<piet::FixedLinearGradient> for LinearGradient {
    fn from(gradient: piet::FixedLinearGradient) -> Self {
        let handle = GradientHandle {
            stops: gradient.stops,
        };
        LinearGradient {
            handle,
            start: gradient.start,
            end: gradient.end,
        }
    }
}

// Hashable, but also contains enough info to create a new gradient texture
#[derive(Debug, Clone)]
pub struct GradientHandle {
    stops: Vec<piet::GradientStop>,
}

// TODO: This should be derived in piet
impl PartialEq for GradientHandle {
    fn eq(&self, other: &GradientHandle) -> bool {
        if self.stops.len() != other.stops.len() {
            return false;
        }
        self.stops
            .iter()
            .zip(other.stops.iter())
            .all(|(a, b)| a.pos == b.pos && a.color.as_rgba_u32() == b.color.as_rgba_u32())
    }
}

impl Eq for GradientHandle {}

impl Hash for GradientHandle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.stops.len().hash(state);
        for stop in &self.stops {
            stop.pos.to_bits().hash(state);
            stop.color.as_rgba_u32().hash(state);
        }
    }
}

pub struct GradientStore {
    gradients: HashMap<GradientHandle, wgpu::Texture>,
}

impl GradientStore {
    pub fn new() -> GradientStore {
        GradientStore {
            gradients: HashMap::new(),
        }
    }

    pub fn get_texture(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        handle: &GradientHandle,
    ) -> &wgpu::Texture {
        self.gradients
            .entry(handle.clone())
            .or_insert_with(|| create_gradient_texture(device, encoder, &handle.stops))
    }
}

fn lerp(a: f64, b: f64, x: f64) -> f64 {
    a + x * (b - a)
}

fn create_gradient_texture(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    stops: &[piet::GradientStop],
) -> wgpu::Texture {
    assert!(!stops.is_empty(), "Gradient must has stops.");

    // Creates a 1D gradient LUT with 256 texels.
    let texture_extent = wgpu::Extent3d {
        width: 256,
        height: 1,
        depth: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        size: texture_extent,
        array_layer_count: 1,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D1,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
    });

    let mut sorted_stops = stops.to_owned();
    sorted_stops.sort_by(|a, b| a.pos.partial_cmp(&b.pos).unwrap());

    // Create gradient LUT
    let mut lut: Vec<u8> = vec![0; 256 * 4];
    for i in 0..256 {
        // Convert to 0 to 1 range
        let x = i as f32 / 255.0;
        // Find the two steps to interpolate between
        let (start, end) = match sorted_stops.iter().position(|stop| stop.pos > x) {
            Some(b) => {
                let a = if b == 0 { 0 } else { b - 1 };
                (&sorted_stops[a], &sorted_stops[b])
            }
            None => (sorted_stops.last().unwrap(), sorted_stops.last().unwrap()),
        };
        // Compute interpolated color
        let norm = if end.pos == start.pos {
            0.0
        } else {
            ((x - start.pos) / (end.pos - start.pos)) as f64
        };
        let (s_r, s_g, s_b, s_a) = split_rgba(&start.color);
        let (e_r, e_g, e_b, e_a) = split_rgba(&end.color);
        lut[i * 4] = (lerp(s_r, e_r, norm) * 255.0) as u8;
        lut[i * 4 + 1] = (lerp(s_g, e_g, norm) * 255.0) as u8;
        lut[i * 4 + 2] = (lerp(s_b, e_b, norm) * 255.0) as u8;
        lut[i * 4 + 3] = (lerp(s_a, e_a, norm) * 255.0) as u8;
    }

    let temp_buf = device.create_buffer_with_data(lut.as_slice(), wgpu::BufferUsage::COPY_SRC);

    // Copy buffer to texture
    encoder.copy_buffer_to_texture(
        wgpu::BufferCopyView {
            buffer: &temp_buf,
            offset: 0,
            row_pitch: 4 * 256,
            image_height: 1,
        },
        wgpu::TextureCopyView {
            texture: &texture,
            mip_level: 0,
            array_layer: 0,
            origin: wgpu::Origin3d::ZERO,
        },
        texture_extent,
    );
    texture
}
