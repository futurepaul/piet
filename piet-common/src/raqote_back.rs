//! Support for piet raqote back-end

use std::marker::PhantomData;

use raqote::{DrawTarget};

use piet::{ErrorKind, ImageFormat};

pub use piet_raqote::*;

pub type Piet<'a> = RaqoteRenderContext<'a>;

pub struct Device;

//QUESTION do I need this phantom data thing?
pub struct BitmapTarget<'a> {
  dt: DrawTarget,
  phantom: PhantomData<&'a ()>
}

impl Device {
  pub fn new() -> Result<Device, piet::Error> {
    Ok(Device)
  }

  //TODO support scale
  pub fn bitmap_target(
    &self,
    width: usize,
    height: usize,
    _pix_scale: f64,
  ) -> Result<BitmapTarget, piet::Error> {
    let dt = DrawTarget::new(width as i32, height as i32);
    let phantom = Default::default();
    Ok(BitmapTarget {
      dt, phantom
    })
  }
}

impl<'a> BitmapTarget<'a> {
  pub fn render_context<'b>(&'b mut self) -> RaqoteRenderContext<'b> {
    RaqoteRenderContext::new(&mut self.dt)
  }

  pub fn into_raw_pixels(mut self, fmt: ImageFormat) -> Result<Vec<u32>, piet::Error> {
    //Only support RGBA currently
    if fmt != ImageFormat::RgbaPremul {
      return Err(piet::new_error(ErrorKind::NotSupported));
    }

    let buf = self.dt.get_data();

    //TODO if we know the width * height we can size this vec upfront
    // let mut output = Vec::new();
    let output = buf;

    // for pixel in buf {
    //         let a = (pixel >> 24) & 0xffu32;
    //         let mut r = (pixel >> 16) & 0xffu32;
    //         let mut g = (pixel >> 8) & 0xffu32;
    //         let mut b = (pixel >> 0) & 0xffu32;

    //         if a > 0u32 {
    //             r = r * 255u32 / a;
    //             g = g * 255u32 / a;
    //             b = b * 255u32 / a;
    //         }

    //         output.push(r as u8);
    //         output.push(g as u8);
    //         output.push(b as u8);
    //         output.push(a as u8);
    //     }
    
    Ok(output.to_vec())

  }
}
