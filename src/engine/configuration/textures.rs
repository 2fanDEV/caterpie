use std::{fs::File, io::Error};

use ash::{
    vk::{
        AccessFlags, BufferMemoryBarrier, BufferUsageFlags, CommandBuffer, CommandPool, DependencyFlags, DeviceMemory, DeviceSize, Extent3D, Format, Image, ImageAspectFlags, ImageCreateFlags, ImageCreateInfo, ImageLayout, ImageMemoryBarrier, ImageSubresourceRange, ImageTiling, ImageType, ImageUsageFlags, MemoryAllocateInfo, MemoryBarrier, MemoryMapFlags, MemoryPropertyFlags, PhysicalDevice, PipelineStageFlags, Queue, QueueFamilyProperties, QueueFlags, SampleCountFlags, SharingMode
    },
    Device, Instance,
};
use log::warn;
use png::BitDepth;

use super::Configuration;

struct Texture {
    width: u32,
    height: u32,
    channels: u32,
    depth: BitDepth,
}

impl Texture {
    fn new(width: u32, height: u32, channels: u32, depth: u8) -> Texture {
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
        let image = png::Decoder::new(match File::open("src/resources/texture.png") {
            Ok(file) => file,
            Err(err) => {
                return Err(err);
            }
        });
        let read_info = image.read_info();
        let (tex_width, tex_height) = read_info.as_ref().unwrap().info().size();
        warn!("{:?}", read_info.as_ref().unwrap().info());
        let image_size: DeviceSize = (tex_width * tex_height * 4).into();
        let b_type = vec![image_size];
        let texture = Texture::new(tex_width, tex_height, 0, 1);

        let (staging_buffer, staging_buffer_memory) = self
            .create_buffer(
                self.instance.as_ref().unwrap(),
                self.physical_device.as_ref().unwrap(),
                device,
                &b_type,
                self.command_pool.as_ref().unwrap(),
                BufferUsageFlags::TRANSFER_SRC,
                MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
                self.graphics_queue.as_ref().unwrap(),
            )
            .unwrap();

        unsafe {
            let data = device
                .map_memory(
                    staging_buffer_memory,
                    0,
                    image_size,
                    MemoryMapFlags::empty(),
                )
                .unwrap();
            device.unmap_memory(staging_buffer_memory);
        };

        let (image, image_memory) = Self::create_image(
            device,
            self.instance.as_ref().unwrap(),
            self.physical_device.unwrap(),
            texture,
            Format::R8G8B8A8_SRGB,
            ImageTiling::OPTIMAL,
            ImageUsageFlags::TRANSFER_DST | ImageUsageFlags::SAMPLED,
            MemoryPropertyFlags::empty(),
        )
        .unwrap();

        self.texture_image = image;
        self.texture_image_memory = image_memory;

        Ok(self)
    }

    fn create_image(
        device: &Device,
        instance: &Instance,
        physical_device: PhysicalDevice,
        texture: Texture,
        format: Format,
        tiling: ImageTiling,
        usage: ImageUsageFlags,
        properties: MemoryPropertyFlags,
    ) -> Result<(Image, DeviceMemory), Error> {
        let image_create_info = ImageCreateInfo::default()
            .image_type(ImageType::TYPE_2D)
            .extent(texture.into())
            .mip_levels(1)
            .array_layers(1)
            .format(format)
            .tiling(tiling)
            .initial_layout(ImageLayout::UNDEFINED)
            .usage(usage)
            .samples(SampleCountFlags::TYPE_1)
            .flags(ImageCreateFlags::empty())
            .sharing_mode(SharingMode::EXCLUSIVE);
        unsafe {
            let image = device.create_image(&image_create_info, None).unwrap();

            let memory_requirements = device.get_image_memory_requirements(image);

            let memory_allocate_info = MemoryAllocateInfo::default()
                .allocation_size(memory_requirements.size)
                .memory_type_index(
                    Self::find_memory_type(
                        instance,
                        physical_device,
                        memory_requirements.memory_type_bits,
                        properties,
                    )
                    .unwrap(),
                );

            let image_memory = device.allocate_memory(&memory_allocate_info, None).unwrap();
            device.bind_image_memory(image, image_memory, 0).unwrap();

            Ok((image, image_memory))
        }
    }

    fn transition_image_layout(
        &self,
        image: Image,
        format: Format,
        old_image_layout: ImageLayout,
        new_image_layout: ImageLayout,
    ) {
        let command = self.single_time_command().unwrap();

        let sub_resource_range = ImageSubresourceRange::default()
            .aspect_mask(ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1);

        let pipeline = vec![ImageMemoryBarrier::default()
            .old_layout(old_image_layout)
            .new_layout(new_image_layout)
            .src_queue_family_index(0)
            .dst_queue_family_index(0)
            .image(image)
            .subresource_range(sub_resource_range)
            .src_access_mask(AccessFlags::empty())
            .dst_access_mask(AccessFlags::empty())];

        let memory_barrier = vec![MemoryBarrier::default()];

        let barrier_memory_barrier = vec![BufferMemoryBarrier::default()];

        unsafe { self.device.as_ref().unwrap().cmd_pipeline_barrier(
            command,
            PipelineStageFlags::empty(),
            PipelineStageFlags::empty(),
            DependencyFlags::empty(),
            &memory_barrier,
            &barrier_memory_barrier,
            &pipeline,
        ) };

        self.end_single_time_command(&command);
    }
}
