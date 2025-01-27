use std::ops::Add;

use ash::vk::{CommandBufferResetFlags, PipelineStageFlags, PresentInfoKHR, SubmitInfo};
use log::{error, info};
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::engine::configuration_builder::MAX_FLIGHT_FENCES;

use crate::Configuration;

pub struct Engine {
    configuration: Configuration,
}

impl Engine {
    pub fn init(window: &Window) -> Result<Engine, &str> {
        let configuration: Configuration = Configuration::default(window);
        Ok(Self { configuration })
    }

    pub fn window_resized(&mut self, size: PhysicalSize<u32>) {
        self.configuration.window_resized(size);
    }

    pub fn draw_frame(&mut self) {
        let current_frame = self.configuration.frame as usize;
        let device = &self.configuration.logical_device.clone();
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

            let next_image_query_result = self.configuration.swapchain_device.acquire_next_image(
                self.configuration.swapchain,
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
            let swapchains = vec![self.configuration.swapchain];

            let submit_info = vec![SubmitInfo::default()
                .wait_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&wait_stages)
                .command_buffers(&command_buffer)
                .signal_semaphores(&signal_semaphores)];
            let image_indices = vec![next_image_index];
            device
                .queue_submit(
                    self.configuration.presentation_queue,
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
                .queue_present(self.configuration.presentation_queue, &present_info)
                .unwrap();

            if self.configuration.resized_flag {
                self.configuration.resized_flag = false;
                self.configuration.recreate_swapchain();
            }

            self.configuration.frame = (self.configuration.frame.add(1)) % MAX_FLIGHT_FENCES;
        };
    }
}
