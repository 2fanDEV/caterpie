use std::ops::Add;
use std::time::Instant;

use ash::vk::{
    Handle, MemoryMapFlags, PipelineStageFlags, PresentInfoKHR, SubmitInfo,
};
use ash::vk::CommandBufferResetFlags;
use cgmath::{perspective, point3, vec3, Deg, Matrix4};
use configuration::buffer_types::uniform_buffer_types::UniformBufferObject;
use log::error;
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::engine::configuration::Configuration;
use crate::engine::configuration::MAX_FLIGHT_FENCES;

mod configuration;
#[derive(Default)]
pub struct Engine {
    configuration: Configuration,
    start: Option<Instant>,
    frame: u32,
}

impl Engine {
    pub fn init(window: &Window) -> Result<Engine, &str> {
        let configuration = Configuration::default()
            .create_instance(window)
            .unwrap()
            .create_surface(window)
            .unwrap()
            .pick_physical_device()
            .unwrap()
            .create_device()
            .unwrap()
            .create_swap_chain()
            .unwrap()
            .create_swapchain_image_views()
            .unwrap()
            .create_render_pass()
            .unwrap()
            .create_descriptor_set_layout()
            .unwrap()
            .create_graphics_pipeline()
            .unwrap()
            .create_framebuffers()
            .unwrap()
            .create_command_pool()
            .unwrap()
            .create_texture_image()
            .unwrap()
            .create_vertex_buffer()
            .unwrap()
            .create_index_buffer()
            .unwrap()
            .create_uniform_buffer()
            .unwrap()
            .create_descriptor_pool()
            .unwrap()
            .create_descriptor_sets()
            .unwrap()
            .create_command_buffer()
            .unwrap()
            .create_sync_objects()
            .unwrap()
            .build();
        Ok(Self {
            configuration,
            start: Some(Instant::now()),
            frame: 0,
        })
    }

    pub fn window_resized(&mut self, size: PhysicalSize<u32>) {
        self.configuration.window_resized(size);
    }

    fn update_uniform_buffer(&mut self, current_image: u32) {
        let time = self.start.unwrap().elapsed().as_secs_f32();

        let device = self.configuration.device.as_ref().unwrap();

        let model = Matrix4::from_axis_angle(vec3(0.0, 0.0, 1.0), Deg(85.0) * time * 2.0);

        let view = Matrix4::look_at_rh(
            point3(2.0, 2.0, 2.0),
            point3(0.0, 0.0, 0.0),
            vec3(0.0, 0.0, 1.0),
        );

        let mut proj = perspective(
            Deg(45.0),
            self.configuration.extent.unwrap().width as f32
                / self.configuration.extent.unwrap().height as f32,
            0.1,
            10.0,
        );

        proj[1][1] *= -1.0;

        let ubo = UniformBufferObject {
            model,
            view,
            projection: proj,
        };
        unsafe {
            let mem = device
                .map_memory(
                    self.configuration.uniform_buffer_memory[current_image as usize],
                    0,
                    size_of::<UniformBufferObject>() as u64,
                    MemoryMapFlags::empty(),
                )
                .unwrap();
            std::ptr::copy_nonoverlapping(&ubo, mem.cast(), 1);

            device.unmap_memory(self.configuration.uniform_buffer_memory[current_image as usize]);
        };
    }

    pub fn draw_frame(&mut self) {
        let current_frame = self.frame as usize;
        let device = self.configuration.device.clone().unwrap();
        let fences = self.configuration.in_flight_fences.clone();
        let command_buffer = self.configuration.command_buffer[current_frame];
        unsafe {
            match device.wait_for_fences(&[fences[current_frame]], true, u64::MAX) {
                Ok(_) => {}
                Err(_) => {
                    error!("Failed to wait for fences! Aborting!");
                    panic!("Failed to wait 4 fences");
                }
            }

            let next_image_query_result = self
                .configuration
                .swapchain_device
                .as_ref()
                .unwrap()
                .acquire_next_image(
                    self.configuration.swapchain.unwrap(),
                    u64::MAX,
                    self.configuration.image_available_semaphores[current_frame],
                    fences[current_frame],
                );

            let mut next_image_index: u32 = 0;
            match next_image_query_result {
                Ok(next_image) => {
                    next_image_index = next_image.0;
                }
                Err(_) => {
                    self.configuration.recreate_swapchain();
                    return;
                }
            }

            device
                .reset_fences(&[fences[current_frame]])
                .expect("Failed to reset fences");

            device
                .reset_command_buffer(command_buffer, CommandBufferResetFlags::default())
                .unwrap();

            self.configuration
                .record_command_buffer(&command_buffer, next_image_index);

            let wait_semaphores =
                vec![self.configuration.image_available_semaphores[current_frame]];
            let signal_semaphores =
                vec![self.configuration.render_finished_semaphores[current_frame]];
            let command_buffer = vec![self.configuration.command_buffer[current_frame]];
            let wait_stages = vec![PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let swapchains = vec![self.configuration.swapchain.unwrap()];

            self.update_uniform_buffer(next_image_index);

            let submit_info = vec![SubmitInfo::default()
                .wait_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&wait_stages)
                .command_buffers(&command_buffer)
                .signal_semaphores(&signal_semaphores)];
            let image_indices = vec![next_image_index];
            device
                .queue_submit(
                    self.configuration.presentation_queue.unwrap(),
                    &submit_info,
                    fences[current_frame],
                )
                .expect("Failed to submit queue");

            let present_info = PresentInfoKHR::default()
                .wait_semaphores(&signal_semaphores)
                .swapchains(&swapchains)
                .image_indices(&image_indices);

            match self
                .configuration
                .swapchain_device
                .as_ref()
                .unwrap()
                .queue_present(
                    self.configuration.presentation_queue.unwrap(),
                    &present_info,
                ) {
                Ok(outdated) => match outdated {
                    true => {
                        return self.configuration.recreate_swapchain();
                    }
                    false => {}
                },
                Err(err) => {
                    error!("Error: {err}");
                    panic!();
                }
            }

            if self.configuration.window_resized {
                self.configuration.window_resized = false;
                self.configuration.recreate_swapchain();
            }

            self.frame = (self.frame.add(1)) % MAX_FLIGHT_FENCES;
        };
    }
}
