use std::{
    ffi::{c_void, CStr, CString},
    io::Cursor,
    path::Path,
};

use ash::vk::{
    AccessFlags, Buffer, BufferCopy, BufferCreateInfo, BufferUsageFlags, ClearColorValue,
    ClearValue, CommandBufferBeginInfo, CommandBufferUsageFlags, DeviceMemory, DeviceSize, Fence,
    FenceCreateFlags, FenceCreateInfo, IndexType, MemoryAllocateInfo, MemoryMapFlags,
    MemoryPropertyFlags, PipelineInputAssemblyStateCreateInfo, PipelineStageFlags,
    RenderPassBeginInfo, Semaphore, SemaphoreCreateFlags, SemaphoreCreateInfo, SubmitInfo,
    SubpassContents, SubpassDependency, SUBPASS_EXTERNAL,
};
use ash::{
    util::read_spv,
    vk::{
        ApplicationInfo, AttachmentDescription, AttachmentLoadOp, AttachmentReference,
        AttachmentStoreOp, BlendFactor, BlendOp, ColorComponentFlags, ColorSpaceKHR, CommandBuffer,
        CommandBufferAllocateInfo, CommandBufferLevel, CommandPool, CommandPoolCreateFlags,
        CommandPoolCreateInfo, ComponentMapping, ComponentSwizzle, CompositeAlphaFlagsKHR,
        CullModeFlags, DebugUtilsMessageSeverityFlagsEXT, DebugUtilsMessageTypeFlagsEXT,
        DebugUtilsMessengerCallbackDataEXT, DebugUtilsMessengerCreateInfoEXT,
        DebugUtilsMessengerEXT, DeviceCreateInfo, DeviceQueueCreateInfo, DynamicState, Extent2D,
        Format, Framebuffer, FramebufferCreateInfo, FrontFace, GraphicsPipelineCreateInfo, Image,
        ImageAspectFlags, ImageLayout, ImageSubresourceRange, ImageUsageFlags, ImageView,
        ImageViewCreateInfo, ImageViewType, InstanceCreateFlags, InstanceCreateInfo, LogicOp,
        Offset2D, PhysicalDevice, PhysicalDeviceFeatures, Pipeline, PipelineBindPoint,
        PipelineCache, PipelineColorBlendAttachmentState, PipelineColorBlendStateCreateInfo,
        PipelineDepthStencilStateCreateInfo, PipelineDynamicStateCreateFlags,
        PipelineDynamicStateCreateInfo, PipelineLayoutCreateInfo,
        PipelineMultisampleStateCreateInfo, PipelineRasterizationStateCreateInfo,
        PipelineShaderStageCreateInfo, PipelineVertexInputStateCreateInfo,
        PipelineViewportStateCreateInfo, PolygonMode, PresentModeKHR, PrimitiveTopology, Queue,
        QueueFlags, Rect2D, RenderPass, RenderPassCreateInfo, SampleCountFlags, ShaderModule,
        ShaderModuleCreateInfo, ShaderStageFlags, SharingMode, SubpassDescription,
        SurfaceFormatKHR, SurfaceKHR, SwapchainCreateInfoKHR, SwapchainKHR, Viewport,
        EXT_DEBUG_UTILS_NAME, KHR_GET_PHYSICAL_DEVICE_PROPERTIES2_NAME,
        KHR_PORTABILITY_ENUMERATION_NAME, KHR_SWAPCHAIN_NAME,
    },
    Device, Entry, Instance,
};
use cgmath::{vec2, vec3};
use log::*;
use winit::{
    dpi::PhysicalSize,
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::Window,
};

use crate::engine::vertex::*;

use crate::utils;

pub const MAX_FLIGHT_FENCES: u32 = 2;

#[allow(clippy::pedantic)]
#[derive(Default, Clone)]
pub struct Configuration {
    vulkan_entry: Option<Entry>,
    instance: Option<Instance>,
    physical_device: Option<PhysicalDevice>,
    physical_device_features: Option<PhysicalDeviceFeatures>,
    queue_family_indices: Option<QueueFamilyIndices>,
    pub logical_device: Option<Device>,
    pub graphics_queue: Option<Queue>,
    pub presentation_queue: Option<Queue>,
    device_extensions: Vec<*const i8>,
    surface_instance: Option<ash::khr::surface::Instance>,
    pub surface: Option<SurfaceKHR>,
    surface_format: Option<SurfaceFormatKHR>,
    present_mode: Option<PresentModeKHR>,
    extent: Option<Extent2D>,
    image_count: u32,
    swapchain_support_details: Option<SwapchainSupportDetails>,
    pub swapchain_device: Option<ash::khr::swapchain::Device>,
    pub swapchain: Option<SwapchainKHR>,
    swapchain_images: Vec<Image>,
    image_views: Vec<ImageView>,
    viewports: Vec<Viewport>,
    scissors: Vec<Rect2D>,

    render_pass: Option<RenderPass>,
    graphics_pipelines: Vec<Pipeline>,

    pub framebuffers: Vec<Framebuffer>,
    pub command_pool: Option<CommandPool>,
    pub command_buffer: Vec<CommandBuffer>,

    pub image_available_semaphores: Vec<Semaphore>,
    pub render_finished_semaphores: Vec<Semaphore>,
    pub in_flight_fences: Vec<Fence>,

    pub vertices: Vec<Vertex>,
    pub vertex_buffer: Buffer,
    indices: Vec<u16>,
    index_buffer: Buffer,
    width: u32,
    height: u32,

    pub window_resized: bool,

    debug_instance: Option<ash::ext::debug_utils::Instance>,
    debug_messenger: Option<DebugUtilsMessengerEXT>,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct QueueFamilyIndices {
    pub graphics_queue: Option<u32>,
    pub presentation_queue: Option<u32>,
}

impl QueueFamilyIndices {
    fn graphics_family_index(&mut self, index: u32) {
        self.graphics_queue = Some(index);
    }

    fn presentation_queue(&mut self, index: u32) {
        self.presentation_queue = Some(index);
    }

    fn is_complete(&self) -> bool {
        self.graphics_queue.is_some() && self.presentation_queue.is_some()
    }

    fn find_queue_family_indices(
        instance: Instance,
        surface_instance: ash::khr::surface::Instance,
        surface: SurfaceKHR,
        physical_device: PhysicalDevice,
    ) -> Option<QueueFamilyIndices> {
        let mut queue_family_indices = QueueFamilyIndices::default();
        unsafe {
            let queue_family_properties =
                instance.get_physical_device_queue_family_properties(physical_device);
            let queue_idx = queue_family_properties
                .iter()
                .enumerate()
                .find(|(idx, &qf)| qf.queue_flags.contains(QueueFlags::GRAPHICS));
            match queue_idx {
                Some(res) => queue_family_indices.graphics_family_index(res.0 as u32),
                None => return Some(queue_family_indices),
            }

            let physical_device_surface_support = surface_instance
                .get_physical_device_surface_support(
                    physical_device,
                    queue_idx.unwrap().0 as u32,
                    surface,
                )
                .unwrap();
            if physical_device_surface_support {
                queue_family_indices.presentation_queue(queue_idx.unwrap().0 as u32);
            }

            Some(queue_family_indices)
        }
    }
}

#[derive(Clone, Debug)]
pub struct SwapchainSupportDetails {
    pub capabilities: ash::vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<ash::vk::SurfaceFormatKHR>,
    pub present_modes: Vec<ash::vk::PresentModeKHR>,
}

impl SwapchainSupportDetails {
    pub fn query_swapchain_support(
        instance: &Instance,
        surface_instance: &ash::khr::surface::Instance,
        surface: &SurfaceKHR,
        physical_device: &PhysicalDevice,
    ) -> SwapchainSupportDetails {
        unsafe {
            let capabilities = surface_instance
                .get_physical_device_surface_capabilities(*physical_device, *surface)
                .unwrap();
            let formats = surface_instance
                .get_physical_device_surface_formats(*physical_device, *surface)
                .unwrap();
            let present_modes = surface_instance
                .get_physical_device_surface_present_modes(*physical_device, *surface)
                .unwrap();
            SwapchainSupportDetails {
                capabilities,
                formats,
                present_modes,
            }
        }
    }

    pub fn choose_swap_chain_format(&self) -> SurfaceFormatKHR {
        let surface_format_khr = self.formats.iter().find(|format| {
            format.format == Format::R8G8B8A8_UNORM
                && format.color_space.eq(&ColorSpaceKHR::SRGB_NONLINEAR)
        });

        if surface_format_khr.is_some() {
            return *surface_format_khr.unwrap();
        } else {
            SurfaceFormatKHR::default()
                .format(Format::R8G8B8A8_UNORM)
                .color_space(ColorSpaceKHR::SRGB_NONLINEAR)
        }
    }

    pub fn choose_present_mode(&self) -> PresentModeKHR {
        let present_mode = self
            .present_modes
            .iter()
            .find(|&present_mode| *present_mode == PresentModeKHR::MAILBOX);
        if present_mode.is_some() {
            return *present_mode.unwrap();
        }

        return PresentModeKHR::FIFO;
    }

    pub fn choose_swap_extent(&self, buffer_width: u32, buffer_height: u32) -> Extent2D {
        if self.capabilities.current_extent.width != u32::max_value() {
            return self.capabilities.current_extent;
        } else {
            let mut extent_2d = Extent2D::default()
                .width(buffer_width)
                .height(buffer_height);
            extent_2d.width = extent_2d.width.clamp(
                self.capabilities.min_image_extent.width,
                self.capabilities.max_image_extent.width,
            );
            extent_2d.height = extent_2d.height.clamp(
                self.capabilities.min_image_extent.height,
                self.capabilities.max_image_extent.height,
            );

            return extent_2d;
        }
    }
}

impl Configuration {
    pub fn default() -> Self {
        return Self {
            width: 1920,
            height: 1080,
            window_resized: false,
            debug_instance: None,
            in_flight_fences: Vec::new(),
            render_finished_semaphores: Vec::new(),
            image_available_semaphores: Vec::new(),
            command_buffer: Vec::new(),
            framebuffers: Vec::new(),
            graphics_pipelines: Vec::new(),
            scissors: Vec::new(),
            viewports: Vec::new(),
            image_views: Vec::new(),
            swapchain_images: Vec::new(),
            logical_device: None,
            swapchain_device: None,
            swapchain_support_details: None,
            surface_instance: None,
            device_extensions: Vec::new(),
            instance: None,
            vulkan_entry: None,
            vertices: Vec::new(),
            indices: Vec::new(),
            ..Default::default()
        };
    }
    pub fn create_instance(&mut self, window: &Window) -> Result<&mut Configuration, &str> {
        unsafe {
            self.vulkan_entry = Some(
                Entry::load_from("/Users/tufan/VulkanSDK/1.3.296.0/macOS/lib/libvulkan.dylib")
                    .expect("Failed to find vulkan library on this machine"),
            );
            let application_version = 1;
            let application_name = CString::new("Caterpie").unwrap();
            let engine_name = CString::new("Caterpie Engine").unwrap();
            let mut debug_messenger_create_info = DebugUtilsMessengerCreateInfoEXT::default()
                .pfn_user_callback(Some(Self::debug_callback))
                .message_severity(
                    DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                        | DebugUtilsMessageSeverityFlagsEXT::WARNING
                        | DebugUtilsMessageSeverityFlagsEXT::ERROR,
                )
                .message_type(
                    DebugUtilsMessageTypeFlagsEXT::GENERAL
                        | DebugUtilsMessageTypeFlagsEXT::VALIDATION
                        | DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                );
            let app_info = ApplicationInfo::default()
                .application_name(&application_name)
                .engine_name(&engine_name)
                .api_version(0)
                .engine_version(1)
                .application_version(application_version);
            let entry_enumerated_instance_extensions = self
                .vulkan_entry
                .as_ref()
                .unwrap()
                .enumerate_instance_extension_properties(None)
                .unwrap();
            let mut instance_extension_properties = ash_window::enumerate_required_extensions(
                window.display_handle().unwrap().as_raw(),
            )
            .unwrap()
            .to_vec();
            instance_extension_properties.push(KHR_PORTABILITY_ENUMERATION_NAME.as_ptr());
            instance_extension_properties.push(KHR_GET_PHYSICAL_DEVICE_PROPERTIES2_NAME.as_ptr());

            for extension in entry_enumerated_instance_extensions {
                instance_extension_properties.push(extension.extension_name.as_ptr());
            }

            match self.check_validation_layer_support() {
            Ok(_) => {
                    instance_extension_properties.push(EXT_DEBUG_UTILS_NAME.as_ptr());},
            Err(_) => error!("ERROR: VALIDATION LAYERS ARE NOT PRESENT ON THIS MACHINE, PROCEEDING WITHOUT SETTING UP DEBUG MESSENGER")
        }
            let instance_create_info = InstanceCreateInfo::default()
                .application_info(&app_info)
                .flags(InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR)
                .enabled_extension_names(&instance_extension_properties)
                .push_next(&mut debug_messenger_create_info);
            self.instance = Some(
                self.vulkan_entry
                    .as_ref()
                    .unwrap()
                    .create_instance(&instance_create_info, None)
                    .unwrap(),
            );

            info!("Instance has been created!");

            self.debug_instance = Some(ash::ext::debug_utils::Instance::new(
                self.vulkan_entry.as_ref().unwrap(),
                self.instance.as_ref().unwrap(),
            ));
            self.debug_messenger = Some(
                self.debug_instance
                    .as_ref()
                    .unwrap()
                    .create_debug_utils_messenger(&debug_messenger_create_info, None)
                    .unwrap(),
            );
            info!("Debug messenger has been created!");
        }
        Ok(self)
    }

    pub fn create_surface(&mut self, window: &Window) -> Result<&mut Configuration, &str> {
        self.surface_instance = Some(ash::khr::surface::Instance::new(
            self.vulkan_entry.as_ref().unwrap(),
            self.instance.as_ref().unwrap(),
        ));
        unsafe {
            self.surface = Some(
                ash_window::create_surface(
                    self.vulkan_entry.as_ref().unwrap(),
                    self.instance.as_ref().unwrap(),
                    window.display_handle().unwrap().as_raw(),
                    window.window_handle().unwrap().as_raw(),
                    None,
                )
                .unwrap(),
            );
        }
        info!("Surface has been created");
        Ok(self)
    }

    pub fn pick_physical_device(&mut self) -> Result<&mut Configuration, &str> {
        unsafe {
            let instance = self.instance.as_ref().unwrap();
            let physical_devices = instance
                .enumerate_physical_devices()
                .expect("Failed to enumerate physical devices");

            let physical_device = physical_devices
                .iter()
                .find(|&p_device| self.is_device_suitable(p_device));
            if physical_device.is_none() {
                error!("No physical device has been found, abort initialization!");
                return Err("Aborting initialization as there were no physical devices found");
            }
            self.physical_device = Some(physical_device.unwrap()).copied();

            Ok(self)
        }
    }

    pub fn is_device_suitable(&mut self, physical_device: &PhysicalDevice) -> bool {
        let queue_family_indices = QueueFamilyIndices::find_queue_family_indices(
            self.instance.as_ref().unwrap().clone(),
            self.surface_instance.as_ref().unwrap().clone(),
            self.surface.unwrap(),
            *physical_device,
        )
        .expect("Failed to gather queue family indices");
        let mut adequate_swapchain = false;
        let extensions_enabled = self.check_device_extension_support(physical_device);
        if extensions_enabled {
            let swapchain_support_details = SwapchainSupportDetails::query_swapchain_support(
                self.instance.as_ref().unwrap(),
                self.surface_instance.as_ref().unwrap(),
                self.surface.as_ref().unwrap(),
                physical_device,
            );
            self.swapchain_support_details = Some(swapchain_support_details.clone());
            adequate_swapchain = !swapchain_support_details.formats.is_empty()
                && !swapchain_support_details.present_modes.is_empty();
        }

        queue_family_indices.is_complete() && extensions_enabled && adequate_swapchain
    }

    pub fn check_device_extension_support(&mut self, physical_device: &PhysicalDevice) -> bool {
        let device_extensions = vec![ash::khr::swapchain::NAME.to_str().unwrap()];
        let mut flag = true;
        unsafe {
            let enumerate_device_extension_properties = self
                .instance
                .as_ref()
                .unwrap()
                .enumerate_device_extension_properties(*physical_device)
                .unwrap();
            let device_extension_properties: Vec<&str> = enumerate_device_extension_properties
                .iter()
                .map(|property| {
                    property
                        .extension_name_as_c_str()
                        .unwrap()
                        .to_str()
                        .unwrap()
                })
                .collect::<Vec<&str>>();

            for extension in device_extensions {
                if !device_extension_properties.contains(&extension) {
                    flag = false;
                }
            }
        }

        if flag {
            self.device_extensions.push(KHR_SWAPCHAIN_NAME.as_ptr());
        }
        flag
    }

    pub fn check_validation_layer_support(&self) -> Result<bool, &str> {
        let validation_layers = vec!["VK_LAYER_KHRONOS_validation"];
        unsafe {
            let available_layers = self
                .vulkan_entry
                .as_ref()
                .unwrap()
                .enumerate_instance_layer_properties()
                .unwrap();
            for layer in validation_layers {
                for available_layer in available_layers.iter() {
                    if layer.eq(available_layer
                        .layer_name_as_c_str()
                        .unwrap()
                        .to_str()
                        .unwrap())
                    {
                        return Ok(true);
                    }
                }
            }
        };
        Err("Validation Layers are not present on this machine")
    }

    pub fn create_logical_device(&mut self) -> Result<&mut Configuration, &str> {
        let instance = self.instance.as_ref().unwrap();
        self.queue_family_indices = QueueFamilyIndices::find_queue_family_indices(
            instance.clone(),
            self.surface_instance.as_ref().unwrap().clone(),
            self.surface.as_ref().unwrap().clone(),
            self.physical_device
                .expect("Couldn't find appropriate queue family indices"),
        );
        unsafe {
            let queue_priorities = [1.0];
            let queue_family_indices = self.queue_family_indices.unwrap();
            let queue_indices = [
                queue_family_indices.graphics_queue.unwrap(),
                queue_family_indices.presentation_queue.unwrap(),
            ];

            self.physical_device_features =
                Some(instance.get_physical_device_features(self.physical_device.unwrap()));

            let mut device_queue_create_infos = Vec::new();
            for queue_index in queue_indices {
                device_queue_create_infos.push(
                    DeviceQueueCreateInfo::default()
                        .queue_family_index(queue_index)
                        .queue_priorities(&queue_priorities),
                );
            }

            let device_create_info = DeviceCreateInfo::default()
                .queue_create_infos(&device_queue_create_infos)
                .enabled_features(self.physical_device_features.as_ref().unwrap())
                .enabled_extension_names(&self.device_extensions);
            self.logical_device = Some(
                instance
                    .create_device(self.physical_device.unwrap(), &device_create_info, None)
                    .unwrap(),
            );

            self.graphics_queue =
                self.find_device_queue(queue_family_indices.graphics_queue.unwrap());
            self.presentation_queue =
                self.find_device_queue(queue_family_indices.presentation_queue.unwrap());
        }
        Ok(self)
    }

    pub fn find_device_queue(&mut self, queue_family_index: u32) -> Option<Queue> {
        unsafe {
            Some(
                self.logical_device
                    .as_ref()
                    .unwrap()
                    .get_device_queue(queue_family_index, 0),
            )
        }
    }

    pub fn create_swap_chain(&mut self) -> Result<&mut Configuration, &str> {
        self.swapchain_support_details = Some(SwapchainSupportDetails::query_swapchain_support(
            self.instance.as_ref().unwrap(),
            self.surface_instance.as_ref().unwrap(),
            self.surface.as_ref().unwrap(),
            self.physical_device.as_ref().unwrap(),
        ));

        self.surface_format = Some(
            self.swapchain_support_details
                .as_ref()
                .unwrap()
                .choose_swap_chain_format(),
        );
        self.present_mode = Some(
            self.swapchain_support_details
                .as_ref()
                .unwrap()
                .choose_present_mode(),
        );
        self.extent = Some(
            self.swapchain_support_details
                .as_ref()
                .unwrap()
                .choose_swap_extent(self.width, self.height),
        );

        self.image_count = self
            .swapchain_support_details
            .as_ref()
            .unwrap()
            .capabilities
            .min_image_count
            + 1;
        let max_image_count = self
            .swapchain_support_details
            .as_ref()
            .unwrap()
            .capabilities
            .max_image_count;
        if max_image_count > 0 && self.image_count > max_image_count {
            self.image_count = max_image_count;
        }

        let queue_families = [
            self.queue_family_indices.unwrap().graphics_queue.unwrap(),
            self.queue_family_indices
                .unwrap()
                .presentation_queue
                .unwrap(),
        ];

        let mut swapchain_create_info = SwapchainCreateInfoKHR::default()
            .surface(self.surface.unwrap())
            .min_image_count(self.image_count)
            .image_format(self.surface_format.unwrap().format)
            .image_color_space(self.surface_format.unwrap().color_space)
            .image_extent(self.extent.unwrap())
            .image_array_layers(1)
            .image_usage(ImageUsageFlags::COLOR_ATTACHMENT)
            .pre_transform(
                self.swapchain_support_details
                    .as_ref()
                    .unwrap()
                    .capabilities
                    .current_transform,
            )
            .composite_alpha(CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(self.present_mode.unwrap())
            .clipped(true);
        //          .old_swapchain(...);

        self.swapchain_device = Some(ash::khr::swapchain::Device::new(
            self.instance.as_ref().unwrap(),
            self.logical_device.as_ref().unwrap(),
        ));

        if queue_families[0] != queue_families[1] {
            swapchain_create_info = swapchain_create_info
                .image_sharing_mode(SharingMode::CONCURRENT)
                .queue_family_indices(&queue_families);
        } else {
            swapchain_create_info =
                swapchain_create_info.image_sharing_mode(SharingMode::EXCLUSIVE);
        }
        unsafe {
            self.swapchain = Some(
                self.swapchain_device
                    .as_ref()
                    .unwrap()
                    .create_swapchain(&swapchain_create_info, None)
                    .expect("Failed to create swapchain"),
            );

            info!("Swapchain created!");
            self.swapchain_images = self
                .swapchain_device
                .as_ref()
                .unwrap()
                .get_swapchain_images(self.swapchain.unwrap())
                .expect("Failed to retrieve swapchain images");
        }
        info!("Swapchain images retrieved");
        Ok(self)
    }

    pub fn create_swapchain_image_views(&mut self) -> Result<&mut Configuration, &str> {
        let device = self.logical_device.as_ref().unwrap();
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
                    .format(self.surface_format.unwrap().format)
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
        Ok(self)
    }

    pub fn create_shader_module<P: AsRef<Path> + std::fmt::Debug + ToString>(
        &mut self,
        path: P,
    ) -> Result<ShaderModule, &str> {
        let device = self.logical_device.as_ref().unwrap();

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

    pub fn create_render_pass(&mut self) -> Result<&mut Configuration, &str> {
        let attachment_description = vec![AttachmentDescription::default()
            .format(self.surface_format.as_ref().unwrap().format)
            .samples(SampleCountFlags::TYPE_1)
            .load_op(AttachmentLoadOp::LOAD)
            .store_op(AttachmentStoreOp::STORE)
            .stencil_load_op(AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(AttachmentStoreOp::DONT_CARE)
            .initial_layout(ImageLayout::UNDEFINED)
            .final_layout(ImageLayout::PRESENT_SRC_KHR)];

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
            self.render_pass = Some(
                self.logical_device
                    .as_ref()
                    .unwrap()
                    .create_render_pass(&render_pass_create_info, None)
                    .unwrap(),
            );
        }
        info!("Renderpass has been initialized!");
        Ok(self)
    }

    pub fn create_graphics_pipeline(&mut self) -> Result<&mut Configuration, &str> {
        let fragment_shader_module = self
            .create_shader_module(Path::new("src/assets/fragment.spv").to_str().unwrap())
            .unwrap();
        let vertex_shader_module = self
            .create_shader_module(Path::new("src/assets/vertices.spv").to_str().unwrap())
            .unwrap();

        self.vertices = vec![
            Vertex::new(vec2(-0.5, -0.5), vec3(1.0, 0.0, 0.0)),
            Vertex::new(vec2(0.5, -0.5), vec3(0.0, 1.0, 0.0)),
            Vertex::new(vec2(0.5, 0.5), vec3(0.0, 0.0, 1.0)),
            Vertex::new(vec2(-0.5, 0.5), vec3(1.0, 1.0, 1.0)),
        ];

        self.indices = vec![0, 1, 2, 2, 3, 0];

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

        let binding_description = Vertex::get_binding_description();
        let attribute_description = Vertex::get_attribute_description();
        let vertex_input_state = PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&binding_description)
            .vertex_attribute_descriptions(&attribute_description);

        let input_assembly_create_info = PipelineInputAssemblyStateCreateInfo::default()
            .topology(PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        self.viewports = vec![Viewport::default()
            .x(0.0)
            .y(0.0)
            .width(self.extent.unwrap().width as f32)
            .height(self.extent.unwrap().height as f32)
            .min_depth(0.0)
            .max_depth(1.0)];

        self.scissors = vec![Rect2D::default()
            .offset(Offset2D::default().x(0).y(0))
            .extent(self.extent.unwrap())];

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
                .color_write_mask(ColorComponentFlags::RGBA)
                .blend_enable(false)
                .src_color_blend_factor(BlendFactor::ONE)
                .dst_color_blend_factor(BlendFactor::ZERO)
                .color_blend_op(BlendOp::ADD)
                .src_alpha_blend_factor(BlendFactor::ONE)
                .dst_alpha_blend_factor(BlendFactor::ZERO)
                .alpha_blend_op(BlendOp::ADD)];

        let color_blend_state_create_info = PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .logic_op(LogicOp::COPY)
            .attachments(&pipeline_color_blend_attachment_state)
            .blend_constants([0.0, 0.0, 0.0, 0.0]); // OPTIONAL

        let pipeline_layout_create_info = PipelineLayoutCreateInfo::default();
        unsafe {
            let pipeline_layout = self
                .logical_device
                .as_ref()
                .unwrap()
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
                .render_pass(self.render_pass.unwrap())
                .base_pipeline_index(-1)
                .layout(pipeline_layout)
                .base_pipeline_handle(Pipeline::null())
                .stages(&pipeline_shader_create_infos)
                .subpass(0)
                .depth_stencil_state(&depth_stencil_state)];

            info!("Graphics Pipeline Create Info created!");
            self.graphics_pipelines = self
                .logical_device
                .as_ref()
                .unwrap()
                .create_graphics_pipelines(
                    PipelineCache::null(),
                    &graphics_pipeline_create_infos,
                    None,
                )
                .unwrap();
        }
        Ok(self)
    }

    pub fn create_framebuffers(&mut self) -> Result<&mut Configuration, &str> {
        let extent = self.extent.unwrap();
        for image_view in self.image_views.clone() {
            let attachments = [image_view];

            let framebuffer_create_info = FramebufferCreateInfo::default()
                .attachments(&attachments)
                .render_pass(self.render_pass.unwrap())
                .width(extent.width)
                .height(extent.height)
                .layers(1);
            unsafe {
                self.framebuffers.push(
                    self.logical_device
                        .as_ref()
                        .unwrap()
                        .create_framebuffer(&framebuffer_create_info, None)
                        .expect("Failed to create framebuffer"),
                );
            }
        }
        info!("Framebuffers created");
        Ok(self)
    }

    pub fn create_command_pool(&mut self) -> Result<&mut Configuration, &str> {
        let queue_family_indices = self.queue_family_indices.unwrap();

        let command_pool_create_info = CommandPoolCreateInfo::default()
            .queue_family_index(queue_family_indices.graphics_queue.unwrap())
            .flags(CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        unsafe {
            self.command_pool = Some(
                self.logical_device
                    .as_ref()
                    .unwrap()
                    .create_command_pool(&command_pool_create_info, None)
                    .unwrap(),
            );
        }
        info!("Command pool has been created");
        Ok(self)
    }

    pub fn create_command_buffer(&mut self) -> Result<&mut Configuration, &str> {
        let command_buffer_allocate_info = CommandBufferAllocateInfo::default()
            .command_pool(self.command_pool.unwrap())
            .level(CommandBufferLevel::PRIMARY)
            .command_buffer_count(MAX_FLIGHT_FENCES);

        self.command_buffer = unsafe {
            self.logical_device
                .as_ref()
                .unwrap()
                .allocate_command_buffers(&command_buffer_allocate_info)
                .unwrap()
        };
        info!("Command Buffers have been allocated");
        Ok(self)
    }

    pub fn create_sync_objects(&mut self) -> Result<&mut Configuration, &str> {
        for i in 0..MAX_FLIGHT_FENCES {
            self.image_available_semaphores
                .push(self.create_semaphore().unwrap());
            self.render_finished_semaphores
                .push(self.create_semaphore().unwrap());
            self.in_flight_fences.push(self.create_fence().unwrap());
        }

        info!("Sync Object (Semaphores, Fences) have been created");
        Ok(self)
    }

    fn create_semaphore(&self) -> Option<Semaphore> {
        let device = self.logical_device.as_ref().unwrap();
        let sci = SemaphoreCreateInfo::default().flags(SemaphoreCreateFlags::default());
        unsafe { Some(device.create_semaphore(&sci, None).unwrap()) }
    }

    fn create_fence(&self) -> Option<Fence> {
        let device = self.logical_device.as_ref().unwrap();
        let fci = FenceCreateInfo::default().flags(FenceCreateFlags::SIGNALED);
        unsafe { Some(device.create_fence(&fci, None).unwrap()) }
    }

    unsafe extern "system" fn debug_callback(
        message_severity: DebugUtilsMessageSeverityFlagsEXT,
        message_type: DebugUtilsMessageTypeFlagsEXT,
        callback_data: *const DebugUtilsMessengerCallbackDataEXT<'_>,
        user_data: *mut c_void,
    ) -> u32 {
        unsafe {
            let p_callback_data = *callback_data;
            let message_id_name = p_callback_data
                .message_id_name_as_c_str()
                .unwrap()
                .to_string_lossy();
            let message_id_number = p_callback_data.message_id_number;
            let message = p_callback_data
                .message_as_c_str()
                .unwrap()
                .to_string_lossy();

            match message_severity {
                DebugUtilsMessageSeverityFlagsEXT::WARNING => {
                    warn!(
                        "{message_type:?} [{message_id_name} ({message_id_number})] : {message}\n"
                    );
                }
                DebugUtilsMessageSeverityFlagsEXT::ERROR => {
                    error!(
                        "{message_type:?} [{message_id_name} ({message_id_number})] : {message}\n"
                    )
                }
                _ => {
                    info!(
                        "{message_type:?} [{message_id_name} ({message_id_number})] : {message}\n"
                    );
                }
                _ => {
                    info!(
                        "{message_type:?} [{message_id_name} ({message_id_number})] : {message}\n"
                    );
                }
            }
        }
        0
    }

    pub fn record_command_buffer(&mut self, command_buffer: &CommandBuffer, image_index: u32) {
        let command_buffer_begin_info =
            CommandBufferBeginInfo::default().flags(CommandBufferUsageFlags::empty());
        let device = self.logical_device.as_ref().unwrap();
        unsafe {
            device
                .begin_command_buffer(*command_buffer, &command_buffer_begin_info)
                .unwrap();
        }
        let framebuffer = self
            .framebuffers
            .get(image_index as usize)
            .expect("Failed to get framebuffer at given image index");

        let clear_color = vec![ClearValue {
            color: ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        }];

        let render_pass_begin_info = RenderPassBeginInfo::default()
            .render_pass(self.render_pass.unwrap())
            .framebuffer(*framebuffer)
            .render_area(
                Rect2D::default()
                    .extent(self.extent.unwrap())
                    .offset(ash::vk::Offset2D { x: 0, y: 0 }),
            )
            .clear_values(&clear_color);
        unsafe {
            device.cmd_begin_render_pass(
                *command_buffer,
                &render_pass_begin_info,
                SubpassContents::INLINE,
            );
            device.cmd_bind_pipeline(
                *command_buffer,
                PipelineBindPoint::GRAPHICS,
                self.graphics_pipelines[0],
            );
            device.cmd_set_viewport(*command_buffer, 0, &self.viewports);
            device.cmd_set_scissor(*command_buffer, 0, &self.scissors);
            device.cmd_bind_pipeline(
                *command_buffer,
                PipelineBindPoint::GRAPHICS,
                self.graphics_pipelines[0],
            );

            let vertex_buffers = vec![self.vertex_buffer];
            let offsets = vec![0];

            device.cmd_bind_vertex_buffers(*command_buffer, 0, &vertex_buffers, &offsets);
            device.cmd_bind_index_buffer(*command_buffer, self.index_buffer, 0, IndexType::UINT16);
            //            device.cmd_draw(*command_buffer, self.vertices.len() as u32, 1, 0, 0);
            device.cmd_draw_indexed(*command_buffer, self.indices.len() as u32, 1, 0, 0, 0);
            device.cmd_end_render_pass(*command_buffer);
            device.end_command_buffer(*command_buffer).unwrap();
        }
    }

    fn find_memory_type(
        instance: &Instance,
        physical_device: PhysicalDevice,
        type_filter: u32,
        properties: MemoryPropertyFlags,
    ) -> Option<u32> {
        unsafe {
            let memory_properties = instance.get_physical_device_memory_properties(physical_device);
            let memory_types = memory_properties.memory_types.to_vec();
            for i in 0..memory_properties.memory_type_count {
                if type_filter & (1 << i) != 0
                    && (memory_types[i as usize].property_flags & properties)
                        != MemoryPropertyFlags::empty()
                {
                    return Some(i);
                }
            }
        }
        None
    }

    fn allocate_buffer(
        instance: &Instance,
        physical_device: PhysicalDevice,
        device: &Device,
        device_size: DeviceSize,
        usage: BufferUsageFlags,
        memory_property_flags: MemoryPropertyFlags,
        buffer_memory: &mut DeviceMemory,
    ) -> Buffer {
        let buffer_create_info = BufferCreateInfo::default()
            .size(device_size)
            .usage(usage)
            .sharing_mode(SharingMode::EXCLUSIVE);

        unsafe {
            let buffer = device.create_buffer(&buffer_create_info, None).unwrap();

            let mem_requirements = device.get_buffer_memory_requirements(buffer);
            let memory_alloc_info = MemoryAllocateInfo::default()
                .allocation_size(mem_requirements.size)
                .memory_type_index(
                    Self::find_memory_type(
                        &instance,
                        physical_device,
                        mem_requirements.memory_type_bits,
                        memory_property_flags,
                    )
                    .expect("FAILED TO FIND MEMORY TYPE"),
                );

            *buffer_memory = device.allocate_memory(&memory_alloc_info, None).unwrap();
            device
                .bind_buffer_memory(buffer, *buffer_memory, 0)
                .unwrap();
            buffer
        }
    }

    pub fn create_buffer<T>(&mut self, buffer_type: Vec<T>) -> Result<Buffer, ()> {
        let instance = self.instance.as_ref().unwrap();
        let physical_device = self.physical_device.unwrap();
        let device = self.logical_device.as_ref().unwrap();
        let buffer_size = (size_of::<T>() * buffer_type.len()) as u64;
        let mut staging_memory = DeviceMemory::default();
        let mut vertex_buffer_memory = DeviceMemory::default();
        let staging_buffer = Self::allocate_buffer(
            &instance,
            physical_device,
            device,
            buffer_size as u64,
            BufferUsageFlags::TRANSFER_SRC,
            MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
            &mut staging_memory,
        );
        unsafe {
            let data = device
                .map_memory(staging_memory, 0, buffer_size, MemoryMapFlags::empty())
                .unwrap();
            buffer_type
                .as_ptr()
                .copy_to_nonoverlapping(data.cast(), buffer_size as usize);

            device.unmap_memory(staging_memory);
            let buffer = Self::allocate_buffer(
                &instance,
                physical_device,
                device,
                buffer_size as u64,
                BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::VERTEX_BUFFER,
                MemoryPropertyFlags::DEVICE_LOCAL,
                &mut vertex_buffer_memory,
            );

            self.copy_buffer(staging_buffer, buffer, buffer_size);

            device.destroy_buffer(staging_buffer, None);
            device.free_memory(staging_memory, None);
            Ok(buffer)
        }
    }

    pub fn create_vertex_buffer(&mut self) -> Result<&mut Configuration, ()> {
        self.vertex_buffer = self.create_buffer(self.vertices.clone())?;
        Ok(self)
    }

    pub fn create_index_buffer(&mut self) -> Result<&mut Configuration, ()> {
        self.index_buffer = self.create_buffer(self.indices.clone())?;
        Ok(self)
    }

    fn copy_buffer(&self, src_buffer: Buffer, dst_buffer: Buffer, size: DeviceSize) {
        let command_buffer_allocate_info = CommandBufferAllocateInfo::default()
            .level(CommandBufferLevel::PRIMARY)
            .command_pool(self.command_pool.unwrap())
            .command_buffer_count(1);

        let device = self.logical_device.as_ref().unwrap();

        unsafe {
            let command_buffer = device
                .allocate_command_buffers(&command_buffer_allocate_info)
                .unwrap();

            let begin_info =
                CommandBufferBeginInfo::default().flags(CommandBufferUsageFlags::ONE_TIME_SUBMIT);

            self.logical_device
                .as_ref()
                .unwrap()
                .begin_command_buffer(command_buffer[0], &begin_info)
                .unwrap();
            let buffer_copy = vec![BufferCopy::default().src_offset(0).dst_offset(0).size(size)];

            device.cmd_copy_buffer(command_buffer[0], src_buffer, dst_buffer, &buffer_copy);

            device.end_command_buffer(command_buffer[0]).unwrap();

            let submit_info = &[SubmitInfo::default().command_buffers(&command_buffer)];
            device
                .queue_submit(self.graphics_queue.unwrap(), submit_info, Fence::null())
                .unwrap();
            device
                .queue_wait_idle(self.graphics_queue.unwrap())
                .unwrap();
            device.free_command_buffers(self.command_pool.unwrap(), &command_buffer);
        };
    }

    pub fn window_resized(&mut self, size: PhysicalSize<u32>) {
        self.window_resized = true;
        self.width = size.width;
        self.height = size.height;
    }

    pub fn build(&mut self) -> Configuration {
        Configuration {
            vulkan_entry: self.vulkan_entry.clone(),
            instance: self.instance.clone(),
            physical_device: self.physical_device,
            physical_device_features: self.physical_device_features,
            queue_family_indices: self.queue_family_indices,
            logical_device: self.logical_device.clone(),
            graphics_queue: self.graphics_queue,
            presentation_queue: self.presentation_queue,
            device_extensions: self.device_extensions.clone(),
            surface_instance: self.surface_instance.clone(),
            surface: self.surface,
            surface_format: self.surface_format,
            present_mode: self.present_mode,
            extent: self.extent,
            image_count: self.image_count,
            swapchain_support_details: self.swapchain_support_details.clone(),
            swapchain_device: self.swapchain_device.clone(),
            swapchain: self.swapchain,
            swapchain_images: self.swapchain_images.clone(),
            image_views: self.image_views.clone(),
            viewports: self.viewports.clone(),
            scissors: self.scissors.clone(),

            render_pass: self.render_pass,
            graphics_pipelines: self.graphics_pipelines.clone(),

            framebuffers: self.framebuffers.clone(),
            command_pool: self.command_pool,
            command_buffer: self.command_buffer.clone(),

            image_available_semaphores: self.image_available_semaphores.clone(),
            render_finished_semaphores: self.render_finished_semaphores.clone(),
            in_flight_fences: self.in_flight_fences.clone(),

            vertices: self.vertices.clone(),
            vertex_buffer: self.vertex_buffer.clone(),
            indices: self.indices.clone(),
            index_buffer: self.index_buffer.clone(),

            width: self.width,
            height: self.height,

            window_resized: self.window_resized,

            debug_instance: self.debug_instance.clone(),
            debug_messenger: self.debug_messenger,
        }
    }

    pub fn recreate_swapchain(&mut self) {
        unsafe {
            self.logical_device
                .as_ref()
                .unwrap()
                .device_wait_idle()
                .unwrap();

            self.destroy_swapchain();
            let _ = self
                .create_swap_chain()
                .unwrap()
                .create_swapchain_image_views()
                .unwrap()
                .create_render_pass()
                .unwrap()
                .create_render_pass()
                .unwrap()
                .create_graphics_pipeline()
                .unwrap()
                .create_framebuffers()
                .unwrap()
                .create_command_buffer();
        }
    }

    fn destroy_swapchain(&mut self) {
        unsafe {
            self.framebuffers.iter().for_each(|f| {
                self.logical_device
                    .as_ref()
                    .unwrap()
                    .destroy_framebuffer(*f, None)
            });
            self.framebuffers.clear();
            self.logical_device
                .as_ref()
                .unwrap()
                .free_command_buffers(self.command_pool.unwrap(), &self.command_buffer);
            self.logical_device
                .as_ref()
                .unwrap()
                .destroy_pipeline(self.graphics_pipelines[0], None);
            self.logical_device
                .as_ref()
                .unwrap()
                .destroy_render_pass(self.render_pass.unwrap(), None);
            self.image_views.iter().for_each(|v| {
                self.logical_device
                    .as_ref()
                    .unwrap()
                    .destroy_image_view(*v, None)
            });
            self.image_views.clear();
            self.swapchain_device
                .as_ref()
                .unwrap()
                .destroy_swapchain(self.swapchain.unwrap(), None);
            self.in_flight_fences
                .resize(self.swapchain_images.len(), Fence::null());
        }
    }
}

impl Drop for Configuration {
    fn drop(&mut self) {
        return;
    }
}
