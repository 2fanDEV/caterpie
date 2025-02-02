use std::ops::Add;

use ash::vk::{CommandBufferResetFlags, PipelineStageFlags, PresentInfoKHR, SubmitInfo};
use log::warn;
use log::{error, info};
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::engine::configuration::MAX_FLIGHT_FENCES;
use crate::engine::configuration::Configuration;

#[derive(Default)]
pub struct Engine {
    configuration: Configuration,
    frame: u32
}

impl Engine {
    pub fn init(window: &Window) -> Result<Engine, &str> {
        let configuration = Configuration::default()
            .create_instance(window).unwrap()
            .create_surface(window).unwrap()
            .pick_physical_device().unwrap()
            .create_logical_device().unwrap()
            .create_swap_chain().unwrap()
            .create_swapchain_image_views().unwrap()
            .create_render_pass().unwrap()
            .create_graphics_pipeline().unwrap()
            .create_framebuffers().unwrap()
            .create_command_pool().unwrap()
            .create_vertex_buffer().unwrap()
            .create_index_buffer().unwrap()
            .create_command_buffer().unwrap()
            .create_sync_objects().unwrap()
            .build();
        Ok(Self {
            configuration,
            frame: 0,
        })
    }

    pub fn window_resized(&mut self, size: PhysicalSize<u32>) {
        self.configuration.window_resized(size);
    }

    pub fn draw_frame(&mut self) {
        let current_frame = self.frame as usize;
        let configuration = &mut self.configuration;
        let device = configuration.logical_device.clone().unwrap();
        let fences = configuration.in_flight_fences.clone();
        let command_buffer = configuration.command_buffer[current_frame];
        unsafe {
            match device.wait_for_fences(&[fences[current_frame]], true, u64::MAX) {
                Ok(_) => {}
                Err(_) => {
                    error!("Failed to wait for fences! Aborting!");
                    panic!("Failed to wait 4 fences");
                }
            }

            let next_image_query_result = configuration.swapchain_device.as_ref().unwrap().acquire_next_image(
                configuration.swapchain.unwrap(),
                u64::MAX,
                configuration.image_available_semaphores[current_frame],
                fences[current_frame],
            );

            let mut next_image_index: u32 = 0;

            match next_image_query_result {
                Ok(next_image) => {
                    next_image_index = next_image.0;
                }
                Err(_) => {
                    configuration.recreate_swapchain();
                    return
                }
            }

            device
                .reset_fences(&[fences[current_frame]])
                .expect("Failed to reset fences");

            device
                .reset_command_buffer(command_buffer, CommandBufferResetFlags::default())
                .unwrap();

            configuration.record_command_buffer(&command_buffer, next_image_index);

            let wait_semaphores =
                vec![configuration.image_available_semaphores[current_frame]];
            let signal_semaphores =
                vec![configuration.render_finished_semaphores[current_frame]];
            let command_buffer = vec![configuration.command_buffer[current_frame]];
            let wait_stages = vec![PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let swapchains = vec![configuration.swapchain.unwrap()];

            let submit_info = vec![SubmitInfo::default()
                .wait_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&wait_stages)
                .command_buffers(&command_buffer)
                .signal_semaphores(&signal_semaphores)];
            let image_indices = vec![next_image_index];
            device
                .queue_submit(
                    configuration.presentation_queue.unwrap(),
                    &submit_info,
                    fences[current_frame],
                )
                .expect("Failed to submit queue");

            let present_info = PresentInfoKHR::default()
                .wait_semaphores(&signal_semaphores)
                .swapchains(&swapchains)
                .image_indices(&image_indices);

            self.configuration
                .swapchain_device
                .as_ref()
                .unwrap()
                .queue_present(self.configuration.presentation_queue.unwrap(), &present_info)
                .unwrap();

            if self.configuration.window_resized {
                self.configuration.window_resized = false;
                self.configuration.recreate_swapchain();
            }

            self.frame = (self.frame.add(1)) % MAX_FLIGHT_FENCES;
        };
    }
}
