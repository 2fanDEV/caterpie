pub mod configuration_builder;
pub mod engine;

use ash::{
    google::user_type,
    vk::{
        CommandBuffer, CommandBufferBeginInfo, CommandPool, DebugUtilsMessengerEXT, Extent2D,
        Framebuffer, Image, ImageView, PhysicalDevice, PhysicalDeviceFeatures, Pipeline,
        PipelineBindPoint, PresentModeKHR, Queue, Rect2D, RenderPass, RenderPassBeginInfo,
        SubpassContents, SurfaceFormatKHR, SurfaceKHR, SwapchainKHR, Viewport,
    },
    Device, Entry, Instance,
};
use configuration_builder::{ConfigurationBuilder, QueueFamilyIndices, SwapchainSupportDetails};
use winit::window::Window;

pub struct Configuration {
    vulkan_entry: Entry,
    instance: Instance,
    physical_device: PhysicalDevice,
    physical_device_features: PhysicalDeviceFeatures,
    queue_family_indices: QueueFamilyIndices,
    logical_device: Device,
    graphics_queue: Queue,
    presentation_queue: Queue,
    device_extensions: Vec<*const i8>,
    surface_instance: ash::khr::surface::Instance,
    surface: SurfaceKHR,
    surface_format: SurfaceFormatKHR,
    present_mode: PresentModeKHR,
    extent: Extent2D,
    image_count: u32,
    swapchain_support_details: SwapchainSupportDetails,
    swapchain_device: ash::khr::swapchain::Device,
    swapchain: SwapchainKHR,
    swapchain_images: Vec<Image>,
    image_views: Vec<ImageView>,
    viewports: Vec<Viewport>,
    scissors: Vec<Rect2D>,

    render_pass: RenderPass,
    graphics_pipelines: Vec<Pipeline>,

    framebuffers: Vec<Framebuffer>,
    command_pool: CommandPool,
    command_buffer: Vec<CommandBuffer>,

    width: u32,
    height: u32,

    debug_instance: ash::ext::debug_utils::Instance,
    debug_messenger: DebugUtilsMessengerEXT,
}

impl Configuration {
    pub fn default(window: &Window) -> Configuration {
        ConfigurationBuilder::default()
            .create_instance(window)
            .expect("Failed to create instance")
            .create_surface(window)
            .expect("Failed to create surface")
            .pick_physical_device()
            .expect("Failed to pick physical device")
            .create_logical_device()
            .expect("Failed to create logical device")
            .create_swap_chain()
            .expect("Failed to create swapchain")
            .create_swapchain_image_views()
            .expect("Failed to create swapchain image views")
            .create_render_pass()
            .expect("Failed to create render pass")
            .create_graphics_pipeline()
            .unwrap()
            .create_framebuffers()
            .unwrap()
            .create_command_pool()
            .unwrap()
            .create_command_buffer()
            .unwrap()
            .build()
            .unwrap()
    }

    pub fn record_command_buffer(&mut self, command_buffer: CommandBuffer, image_index: u32) {
        let command_buffer_begin_info = CommandBufferBeginInfo::default();
        let command_buffer = self.command_buffer[0];
        unsafe {
            self.logical_device
                .begin_command_buffer(command_buffer, &command_buffer_begin_info)
                .unwrap();
        }
        let framebuffer = self
            .framebuffers
            .get(image_index as usize)
            .expect("Failed to get framebuffer at given image index");

        let render_pass_begin_info = RenderPassBeginInfo::default()
            .render_pass(self.render_pass)
            .framebuffer(*framebuffer)
            .render_area(
                Rect2D::default()
                    .extent(self.extent)
                    .offset(ash::vk::Offset2D { x: 0, y: 0 }),
            );
        unsafe {
            self.logical_device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin_info,
                SubpassContents::INLINE,
            );
            self.logical_device.cmd_bind_pipeline(
                command_buffer,
                PipelineBindPoint::GRAPHICS,
                self.graphics_pipelines[0],
            );
            self.logical_device
                .cmd_set_viewport(command_buffer, 0, &self.viewports);
            self.logical_device
                .cmd_set_scissor(command_buffer, 0, &self.scissors);
            self.logical_device.cmd_draw(command_buffer, 3, 1, 0, 0);
            self.logical_device.cmd_end_rendering(command_buffer);
            self.logical_device.end_command_buffer(command_buffer).unwrap();
        }
    }
}
