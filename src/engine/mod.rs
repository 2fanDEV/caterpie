pub mod configuration_builder;
pub mod engine;

use std::{ffi::CStr, io::Cursor, path::Path};

use ash::{
    util::read_spv,
    vk::{
        AccessFlags, AttachmentDescription, AttachmentLoadOp, AttachmentReference,
        AttachmentStoreOp, BlendFactor, BlendOp, ClearColorValue, ClearValue, ColorComponentFlags,
        CommandBuffer, CommandBufferAllocateInfo, CommandBufferBeginInfo, CommandBufferLevel,
        CommandBufferUsageFlags, CommandPool, ComponentMapping, ComponentSwizzle,
        CompositeAlphaFlagsKHR, CullModeFlags, DebugUtilsMessengerEXT, DynamicState, Extent2D,
        Fence, Framebuffer, FramebufferCreateInfo, FrontFace, GraphicsPipelineCreateInfo, Image,
        ImageAspectFlags, ImageLayout, ImageSubresourceRange, ImageUsageFlags, ImageView,
        ImageViewCreateInfo, ImageViewType, LogicOp, Offset2D, PhysicalDevice,
        PhysicalDeviceFeatures, Pipeline, PipelineBindPoint, PipelineCache,
        PipelineColorBlendAttachmentState, PipelineColorBlendStateCreateInfo,
        PipelineDepthStencilStateCreateInfo, PipelineDynamicStateCreateFlags,
        PipelineDynamicStateCreateInfo, PipelineInputAssemblyStateCreateInfo,
        PipelineLayoutCreateInfo, PipelineMultisampleStateCreateInfo,
        PipelineRasterizationStateCreateInfo, PipelineShaderStageCreateInfo, PipelineStageFlags,
        PipelineVertexInputStateCreateInfo, PipelineViewportStateCreateInfo, PolygonMode,
        PresentModeKHR, PrimitiveTopology, Queue, Rect2D, RenderPass, RenderPassBeginInfo,
        RenderPassCreateInfo, SampleCountFlags, Semaphore, ShaderModule, ShaderModuleCreateInfo,
        ShaderStageFlags, SharingMode, SubpassContents, SubpassDependency, SubpassDescription,
        SurfaceFormatKHR, SurfaceKHR, SwapchainCreateInfoKHR, SwapchainKHR, Viewport,
        SUBPASS_EXTERNAL,
    },
    Device, Entry, Instance,
};
use configuration_builder::{
    ConfigurationBuilder, QueueFamilyIndices, SwapchainSupportDetails, MAX_FLIGHT_FENCES,
};
use log::{error, info};
use winit::{
    dpi::{PhysicalSize, Size},
    keyboard::Key,
    window::Window,
};

use crate::utils;

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

    image_available_semaphores: Vec<Semaphore>,
    render_finished_semaphores: Vec<Semaphore>,
    in_flight_fences: Vec<Fence>,

    width: u32,
    height: u32,

    debug_instance: ash::ext::debug_utils::Instance,
    debug_messenger: DebugUtilsMessengerEXT,

    frame: u32,
    resized_flag: bool,
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
            .create_sync_objects()
            .unwrap()
            .build()
            .unwrap()
    }

    pub fn window_resized(&mut self, size: PhysicalSize<u32>) {
        self.resized_flag = true;
        self.width = size.width;
        self.height = size.width;
    }

    pub fn record_command_buffer(&self, command_buffer: &CommandBuffer, image_index: u32) {
        let command_buffer_begin_info =
            CommandBufferBeginInfo::default().flags(CommandBufferUsageFlags::empty());
        unsafe {
            self.logical_device
                .begin_command_buffer(*command_buffer, &command_buffer_begin_info)
                .unwrap();
        }
        let framebuffer = self
            .framebuffers
            .get(image_index as usize)
            .expect("Failed to get framebuffer at given image index");

        let clear_color = vec![ClearValue {
            color: ClearColorValue {
                float32: [0.0, 0.0, 0.0, 0.0],
            },
        }];

        let render_pass_begin_info = RenderPassBeginInfo::default()
            .render_pass(self.render_pass)
            .framebuffer(*framebuffer)
            .render_area(
                Rect2D::default()
                    .extent(self.extent)
                    .offset(ash::vk::Offset2D { x: 0, y: 0 }),
            )
            .clear_values(&clear_color);
        unsafe {
            self.logical_device.cmd_begin_render_pass(
                *command_buffer,
                &render_pass_begin_info,
                SubpassContents::INLINE,
            );
            self.logical_device.cmd_bind_pipeline(
                *command_buffer,
                PipelineBindPoint::GRAPHICS,
                self.graphics_pipelines[0],
            );
            self.logical_device
                .cmd_set_viewport(*command_buffer, 0, &self.viewports);
            self.logical_device
                .cmd_set_scissor(*command_buffer, 0, &self.scissors);
            self.logical_device.cmd_draw(*command_buffer, 3, 1, 0, 0);
            self.logical_device.cmd_end_render_pass(*command_buffer);
            self.logical_device
                .end_command_buffer(*command_buffer)
                .unwrap();
        }
    }

    pub fn recreate_swapchain(&mut self) {
        unsafe {
            self.logical_device.device_wait_idle().unwrap();
            self.destroy_swapchain();
            self.recreate_swap_chain();

            self.recreate_swapchain_image_views();
            self.recreate_render_pass();

            self.recreate_graphics_pipeline();
            self.recreate_framebuffers();

            self.recreate_command_buffer();
        }
    }

    fn destroy_swapchain(&mut self) {
        unsafe {
            self.framebuffers
                .iter()
                .for_each(|f| self.logical_device.destroy_framebuffer(*f, None));
            self.framebuffers.clear();
            self.logical_device
                .free_command_buffers(self.command_pool, &self.command_buffer);
            self.logical_device
                .destroy_pipeline(self.graphics_pipelines[0], None);
            self.logical_device
                .destroy_render_pass(self.render_pass, None);
            self.image_views
                .iter()
                .for_each(|v| self.logical_device.destroy_image_view(*v, None));
            self.image_views.clear();
            self.swapchain_device
                .destroy_swapchain(self.swapchain, None);
            self.in_flight_fences
                .resize(self.swapchain_images.len(), Fence::null());
        }
    }

    fn recreate_swap_chain(&mut self) {
        self.swapchain_support_details = SwapchainSupportDetails::query_swapchain_support(
            &self.instance,
            &self.surface_instance,
            &self.surface,
            &self.physical_device,
        );

        self.surface_format = self.swapchain_support_details.choose_swap_chain_format();
        self.present_mode = self.swapchain_support_details.choose_present_mode();

        self.extent = self
            .swapchain_support_details
            .choose_swap_extent(self.width, self.height);

        self.image_count = self.swapchain_support_details.capabilities.min_image_count + 1;
        let max_image_count = self.swapchain_support_details.capabilities.max_image_count;
        if max_image_count > 0 && self.image_count > max_image_count {
            self.image_count = max_image_count;
        }
        let mut swapchain_create_info = SwapchainCreateInfoKHR::default()
            .surface(self.surface)
            .min_image_count(self.image_count)
            .image_format(self.surface_format.format)
            .image_color_space(self.surface_format.color_space)
            .image_extent(self.extent)
            .image_array_layers(1)
            .image_usage(ImageUsageFlags::COLOR_ATTACHMENT)
            .pre_transform(
                self.swapchain_support_details
                    .capabilities
                    .current_transform,
            )
            .composite_alpha(CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(self.present_mode)
            .clipped(true);

        let queue_families = [
            self.queue_family_indices.graphics_queue.unwrap(),
            self.queue_family_indices.presentation_queue.unwrap(),
        ];
        if queue_families[0] != queue_families[1] {
            swapchain_create_info = swapchain_create_info
                .image_sharing_mode(SharingMode::CONCURRENT)
                .queue_family_indices(&queue_families);
        } else {
            swapchain_create_info =
                swapchain_create_info.image_sharing_mode(SharingMode::EXCLUSIVE);
        }
        unsafe {
            self.swapchain = self
                .swapchain_device
                .create_swapchain(&swapchain_create_info, None)
                .expect("Failed to create swapchain");

            self.swapchain_images = self
                .swapchain_device
                .get_swapchain_images(self.swapchain)
                .expect("Failed to retrieve swapchain images");
        }
    }

    fn recreate_swapchain_image_views(&mut self) {
        let device = self.logical_device.clone();
        let component_mapping = ComponentMapping::default()
            .r(ComponentSwizzle::IDENTITY)
            .g(ComponentSwizzle::IDENTITY)
            .b(ComponentSwizzle::IDENTITY)
            .a(ComponentSwizzle::IDENTITY);

        let subresource_range = ImageSubresourceRange::default()
            .aspect_mask(ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1);

        self.image_views = self
            .swapchain_images
            .iter()
            .map(|image| {
                let image_view_create_info = ImageViewCreateInfo::default()
                    .image(*image)
                    .format(self.surface_format.format)
                    .view_type(ImageViewType::TYPE_2D)
                    .components(component_mapping)
                    .subresource_range(subresource_range);
                unsafe {
                    device
                        .create_image_view(&image_view_create_info, None)
                        .expect("Failed to create image view")
                }
            })
            .collect::<Vec<ImageView>>();
    }

    fn recreate_framebuffers(&mut self) {
        for image_view in self.image_views.clone() {
            let attachments = [image_view];

            let framebuffer_create_info = FramebufferCreateInfo::default()
                .attachments(&attachments)
                .render_pass(self.render_pass)
                .width(self.extent.width)
                .height(self.extent.height)
                .layers(1);
            unsafe {
                self.framebuffers.push(
                    self.logical_device
                        .create_framebuffer(&framebuffer_create_info, None)
                        .expect("Failed to create framebuffer"),
                );
            }
        }
    }

    pub fn recreate_render_pass(&mut self) {
        let vec = vec![AttachmentDescription::default()
            .format(self.surface_format.format)
            .samples(SampleCountFlags::TYPE_1)
            .load_op(AttachmentLoadOp::LOAD)
            .store_op(AttachmentStoreOp::STORE)
            .stencil_load_op(AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(AttachmentStoreOp::DONT_CARE)
            .initial_layout(ImageLayout::UNDEFINED)
            .final_layout(ImageLayout::PRESENT_SRC_KHR)];
        let vec = vec;
        let attachment_description = vec;

        let attachment_reference = vec![AttachmentReference::default()
            .attachment(0)
            .layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)];

        let subpass_description = vec![SubpassDescription::default()
            .pipeline_bind_point(PipelineBindPoint::GRAPHICS)
            .color_attachments(&attachment_reference)];

        let subpass_dependency = vec![SubpassDependency::default()
            .src_subpass(SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_stage_mask(PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(AccessFlags::empty())
            .dst_access_mask(AccessFlags::COLOR_ATTACHMENT_WRITE)];

        let render_pass_create_info = RenderPassCreateInfo::default()
            .attachments(&attachment_description)
            .subpasses(&subpass_description)
            .dependencies(&subpass_dependency);

        unsafe {
            self.render_pass = self
                .logical_device
                .create_render_pass(&render_pass_create_info, None)
                .unwrap();
        }
    }

    pub fn create_shader_module<P: AsRef<Path> + std::fmt::Debug + ToString>(
        &mut self,
        path: P,
    ) -> Result<ShaderModule, &str> {
        let device = self.logical_device.clone();

        let shader_binding = utils::io::read_file(&path).unwrap();
        let mut shader_as_byte_arr = Cursor::new(&shader_binding);
        let shader_spv: Vec<u32> =
            read_spv(&mut shader_as_byte_arr).expect("Failed to convert shader shader to spv");

        let shader_spv_c_info = ShaderModuleCreateInfo::default().code(&shader_spv);

        unsafe {
            let shader_module = device.create_shader_module(&shader_spv_c_info, None);

            match shader_module {
                Ok(module) => Ok(module),
                Err(_) => {
                    error!("Failed to create shader module with path {:?}", path);
                    Err("Failed to create shader module")
                }
            }
        }
    }

    pub fn recreate_command_buffer(&mut self) {
        let command_buffer_allocate_info = CommandBufferAllocateInfo::default()
            .command_pool(self.command_pool)
            .level(CommandBufferLevel::PRIMARY)
            .command_buffer_count(MAX_FLIGHT_FENCES);

        self.command_buffer = unsafe {
            self.logical_device
                .allocate_command_buffers(&command_buffer_allocate_info)
                .unwrap()
        };
    }

    fn recreate_graphics_pipeline(&mut self) {
        let fragment_shader_module = self
            .create_shader_module(Path::new("src/assets/fragment.spv").to_str().unwrap())
            .unwrap();
        let vertex_shader_module = self
            .create_shader_module(Path::new("src/assets/vertices.spv").to_str().unwrap())
            .unwrap();

        let name_main: &CStr = c"main";
        let frag_shader_create_info = PipelineShaderStageCreateInfo::default()
            .module(fragment_shader_module)
            .stage(ShaderStageFlags::FRAGMENT)
            .name(name_main);

        let vert_shader_create_info = PipelineShaderStageCreateInfo::default()
            .module(vertex_shader_module)
            .stage(ShaderStageFlags::VERTEX)
            .name(name_main);

        let pipeline_shader_create_infos = vec![vert_shader_create_info, frag_shader_create_info];

        let dynamic_states = vec![DynamicState::VIEWPORT, DynamicState::SCISSOR];

        let vertex_input_state = PipelineVertexInputStateCreateInfo::default();

        let input_assembly_create_info = PipelineInputAssemblyStateCreateInfo::default()
            .topology(PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        self.viewports = vec![Viewport::default()
            .x(0.0)
            .y(0.0)
            .width(self.extent.width as f32)
            .height(self.extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0)];
        self.scissors = vec![Rect2D::default()
            .offset(Offset2D::default().x(0).y(0))
            .extent(self.extent)];
        let pipeline_dynamic_states_create_info = PipelineDynamicStateCreateInfo::default()
            .dynamic_states(&dynamic_states)
            .flags(PipelineDynamicStateCreateFlags::empty());

        let viewport_state = PipelineViewportStateCreateInfo::default()
            .viewports(&self.viewports)
            .scissors(&self.scissors);

        let rasterizer_create_info = PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(CullModeFlags::BACK)
            .front_face(FrontFace::CLOCKWISE)
            .depth_bias_enable(false)
            .depth_bias_constant_factor(0.0)
            .depth_bias_clamp(0.0)
            .depth_bias_slope_factor(0.0);

        let pipeline_multisample_state_create_info = PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(SampleCountFlags::TYPE_1)
            .min_sample_shading(1.0)
            .alpha_to_coverage_enable(false)
            .alpha_to_one_enable(false);

        let pipeline_color_blend_attachment_state =
            vec![PipelineColorBlendAttachmentState::default()
                .color_write_mask(
                    ColorComponentFlags::R
                        | ColorComponentFlags::G
                        | ColorComponentFlags::B
                        | ColorComponentFlags::A,
                )
                .blend_enable(true)
                .src_color_blend_factor(BlendFactor::ONE)
                .dst_color_blend_factor(BlendFactor::ZERO)
                .color_blend_op(BlendOp::ADD)
                .src_alpha_blend_factor(BlendFactor::ONE)
                .dst_alpha_blend_factor(BlendFactor::ZERO)
                .alpha_blend_op(BlendOp::ADD)];

        let color_blend_state_create_info = PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .logic_op(LogicOp::COPY)
            .attachments(&pipeline_color_blend_attachment_state);

        let pipeline_layout_create_info = PipelineLayoutCreateInfo::default();
        unsafe {
            let pipeline_layout = self
                .logical_device
                .create_pipeline_layout(&pipeline_layout_create_info, None)
                .unwrap();

            let depth_stencil_state = PipelineDepthStencilStateCreateInfo::default();

            let graphics_pipeline_create_infos = vec![GraphicsPipelineCreateInfo::default()
                .vertex_input_state(&vertex_input_state)
                .input_assembly_state(&input_assembly_create_info)
                .viewport_state(&viewport_state)
                .rasterization_state(&rasterizer_create_info)
                .multisample_state(&pipeline_multisample_state_create_info)
                .color_blend_state(&color_blend_state_create_info)
                .dynamic_state(&pipeline_dynamic_states_create_info)
                .render_pass(self.render_pass)
                .base_pipeline_index(-1)
                .layout(pipeline_layout)
                .base_pipeline_handle(Pipeline::null())
                .stages(&pipeline_shader_create_infos)
                .subpass(0)
                .depth_stencil_state(&depth_stencil_state)];

            self.graphics_pipelines = self
                .logical_device
                .create_graphics_pipelines(
                    PipelineCache::null(),
                    &graphics_pipeline_create_infos,
                    None,
                )
                .unwrap();
        }
    }
}
