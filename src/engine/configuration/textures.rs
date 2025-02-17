use std::{
    borrow::BorrowMut,
    fs::File,
    io::{Error, ErrorKind},
};

use anyhow::anyhow;
use ash::{
    vk::{
        self, AccessFlags, BorderColor, Buffer, BufferImageCopy, BufferMemoryBarrier,
        BufferUsageFlags, CommandBuffer, CommandPool, CompareOp, DependencyFlags, DeviceMemory,
        DeviceSize, Extent3D, Filter, Format, Image, ImageAspectFlags, ImageCreateFlags,
        ImageCreateInfo, ImageLayout, ImageMemoryBarrier, ImageSubresourceLayers,
        ImageSubresourceRange, ImageTiling, ImageType, ImageUsageFlags, ImageView,
        ImageViewCreateInfo, ImageViewType, MemoryAllocateInfo, MemoryBarrier, MemoryMapFlags,
        MemoryPropertyFlags, Offset3D, PhysicalDevice, PipelineStageFlags, Queue,
        QueueFamilyProperties, QueueFlags, SampleCountFlags, SamplerAddressMode, SamplerCreateInfo,
        SamplerMipmapMode, SharingMode, QUEUE_FAMILY_IGNORED,
    },
    Device, Instance,
};
use log::{debug, info, warn};
use png::BitDepth;

use crate::engine::configuration::QueueFamilyIndices;

use super::Configuration;

#[derive(Debug, Clone, Copy)]
pub struct Texture {
    width: u32,
    height: u32,
    channels: u32,
    depth: BitDepth,
}

impl Texture {
    pub fn new(width: u32, height: u32, channels: u32, depth: u8) -> Texture {
        Self {
            width,
            height,
            channels,
            depth: match BitDepth::from_u8(depth) {
                Some(depth) => depth,
                None => BitDepth::One,
            },
        }
    }
}

impl Into<Extent3D> for Texture {
    fn into(self) -> Extent3D {
        Extent3D::default()
            .depth(self.depth as u32)
            .height(self.height)
            .width(self.width)
    }
}

impl Configuration {
    pub fn create_texture_image(&mut self) -> Result<&mut Configuration, Error> {
        let device = self.device.as_ref().unwrap();
        let image = png::Decoder::new(match File::open("src/resources/viking_room.png") {
            Ok(file) => file,
            Err(err) => {
                return Err(err);
            }
        });
        let mut read_info = image.read_info()?;
        let (tex_width, tex_height) = read_info.info().size();
        let mut pixels = vec![0; read_info.info().raw_bytes()];
        read_info.next_frame(&mut pixels)?;
        let texture = Texture::new(tex_width, tex_height, 0, 1);
        let buffer_size = vec![read_info.info().raw_bytes() as u64];
        let mut staging_buffer_memory: DeviceMemory = DeviceMemory::null();
        let staging_buffer = Self::allocate_buffer(
            self.instance.as_ref().unwrap(),
            self.physical_device.unwrap(),
            device,
            buffer_size[0],
            BufferUsageFlags::TRANSFER_SRC,
            MemoryPropertyFlags::HOST_COHERENT | MemoryPropertyFlags::HOST_VISIBLE,
            &mut staging_buffer_memory,
        );

        unsafe {
            let data = device
                .map_memory(
                    staging_buffer_memory,
                    0,
                    buffer_size[0],
                    MemoryMapFlags::empty(),
                )
                .unwrap();
            std::ptr::copy_nonoverlapping(pixels.as_ptr(), data.cast(), pixels.len());
            device.unmap_memory(staging_buffer_memory);
        }

        let (image, image_memory) = self
            .create_image(
                texture,
                Format::R8G8B8A8_SRGB,
                ImageTiling::OPTIMAL,
                ImageUsageFlags::TRANSFER_DST | ImageUsageFlags::SAMPLED,
                MemoryPropertyFlags::DEVICE_LOCAL,
            )
            .unwrap();

        self.texture_image = image;
        self.texture_image_memory = image_memory;

        self.transition_image_layout(
            image,
            Format::R8G8B8A8_SRGB,
            ImageLayout::UNDEFINED,
            ImageLayout::TRANSFER_DST_OPTIMAL,
        )
        .unwrap();
        self.copy_buffer_to_image(staging_buffer, image, texture);
        self.transition_image_layout(
            image,
            Format::R8G8B8A8_SRGB,
            ImageLayout::TRANSFER_DST_OPTIMAL,
            ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        )
        .unwrap();
        unsafe {
            device.destroy_buffer(staging_buffer, None);
            device.free_memory(staging_buffer_memory, None)
        };
        info!("Texture Image has been created");
        Ok(self)
    }

    pub fn create_texture_image_view(&mut self) -> Result<&mut Configuration, ()> {
        self.texture_image_view = self
            .clone()
            .create_image_view(
                &self.texture_image,
                Format::R8G8B8A8_SRGB,
                ImageAspectFlags::COLOR,
            )
            .unwrap();
        debug!("Texture Image View created");
        Ok(self)
    }

    pub fn create_texture_sampler(&mut self) -> Result<&mut Configuration, ()> {
        let device = self.device.as_ref().unwrap();
        let properties = unsafe {
            self.instance
                .as_ref()
                .unwrap()
                .get_physical_device_properties(self.physical_device.unwrap())
        };

        let sampler_info = SamplerCreateInfo::default()
            .mag_filter(Filter::LINEAR)
            .min_filter(Filter::LINEAR)
            .address_mode_u(SamplerAddressMode::REPEAT)
            .address_mode_v(SamplerAddressMode::REPEAT)
            .address_mode_w(SamplerAddressMode::REPEAT)
            .anisotropy_enable(true)
            .max_anisotropy(properties.limits.max_sampler_anisotropy)
            .border_color(BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(false)
            .compare_enable(false)
            .compare_op(CompareOp::ALWAYS)
            .mipmap_mode(SamplerMipmapMode::LINEAR)
            .mip_lod_bias(0.0)
            .min_lod(0.0)
            .max_lod(0.0);

        self.texture_sampler = unsafe { device.create_sampler(&sampler_info, None).unwrap() };
        debug!("Texture Sampler created");
        Ok(self)
    }
}
