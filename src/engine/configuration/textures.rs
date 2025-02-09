use std::fs::File;

use ash::vk::TextureLODGatherFormatPropertiesAMD;

use super::Configuration;

struct Texture {
    width: u32,
    height: u32,
    channels: u32
}

impl Texture {
    fn new(width: u32, height: u32, channels: u32) -> Texture {
        Self {
            width,
            height,
            channels
        }
    }
}

impl Configuration {

    fn create_texture_image(&mut self) -> Result<&mut Configuration, ()> {
        let device = self.device.as_ref().unwrap();
        png::Decoder::new(File::open("resources/texture.png"))?;
        Ok(self)
    }

}
