use std::{
    ffi::{c_void, CStr, CString},
    io::Cursor,
    path::Path,
};

use ash::{
    util::read_spv,
    vk::{
        ApplicationInfo, BlendFactor, BlendOp, ColorComponentFlags, ColorSpaceKHR, ComponentMapping, ComponentSwizzle, CompositeAlphaFlagsKHR, CullModeFlags, DebugUtilsMessageSeverityFlagsEXT, DebugUtilsMessageTypeFlagsEXT, DebugUtilsMessengerCallbackDataEXT, DebugUtilsMessengerCreateInfoEXT, DebugUtilsMessengerEXT, DeviceCreateInfo, DeviceQueueCreateInfo, DynamicState, Extent2D, Format, FrontFace, Image, ImageAspectFlags, ImageSubresourceRange, ImageUsageFlags, ImageView, ImageViewCreateInfo, ImageViewType, InstanceCreateFlags, InstanceCreateInfo, LogicOp, Offset2D, PhysicalDevice, PhysicalDeviceFeatures, PipelineColorBlendAttachmentState, PipelineColorBlendStateCreateInfo, PipelineDynamicStateCreateFlags, PipelineDynamicStateCreateInfo, PipelineInputAssemblyStateCreateInfo, PipelineLayout, PipelineLayoutCreateInfo, PipelineMultisampleStateCreateInfo, PipelineRasterizationStateCreateInfo, PipelineShaderStageCreateInfo, PipelineViewportStateCreateInfo, PolygonMode, PresentModeKHR, PrimitiveTopology, Queue, QueueFlags, Rect2D, SampleCountFlags, ShaderModule, ShaderModuleCreateInfo, ShaderStageFlags, SharingMode, SurfaceFormatKHR, SurfaceKHR, SwapchainCreateInfoKHR, SwapchainKHR, Viewport, EXT_DEBUG_UTILS_NAME, KHR_GET_PHYSICAL_DEVICE_PROPERTIES2_NAME, KHR_PORTABILITY_ENUMERATION_NAME, KHR_SWAPCHAIN_NAME
    },
    Device, Entry, Instance
};
use log::*;
use winit::{raw_window_handle::{HasDisplayHandle, HasWindowHandle}, window::Window}; 

use crate::utils;

#[allow(clippy::pedantic)]

pub struct Configuration {
    vulkan_entry: Entry,
    instance: Instance,
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
            .build()
            .unwrap()
    }
}

#[derive(Default)]
pub struct ConfigurationBuilder {
    vulkan_entry: Option<Entry>,
    instance: Option<Instance>,
    physical_device: Option<PhysicalDevice>,
    physical_device_features: Option<PhysicalDeviceFeatures>,
    queue_family_indices: Option<QueueFamilyIndices>,
    logical_device: Option<Device>,
    graphics_queue: Option<Queue>,
    presentation_queue: Option<Queue>,
    device_extensions: Vec<*const i8>,
    surface_instance: Option<ash::khr::surface::Instance>,
    surface: Option<SurfaceKHR>,
    surface_format: Option<SurfaceFormatKHR>,
    present_mode: Option<PresentModeKHR>,
    extent: Option<Extent2D>,
    image_count: u32,
    swapchain_support_details: Option<SwapchainSupportDetails>,
    swapchain_device: Option<ash::khr::swapchain::Device>,
    swapchain: Option<SwapchainKHR>,
    swapchain_images: Vec<Image>,
    image_views: Vec<ImageView>,
    viewports: Vec<Viewport>,
    scissors: Vec<Rect2D>,

    width: Option<u32>,
    height: Option<u32>,

    debug_instance: Option<ash::ext::debug_utils::Instance>,
    debug_messenger: Option<DebugUtilsMessengerEXT>,
}

#[derive(Default, Debug, Clone, Copy)]
struct QueueFamilyIndices {
    graphics_queue: Option<u32>,
    presentation_queue: Option<u32>,
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

#[derive(Clone)]
struct SwapchainSupportDetails {
    capabilities: ash::vk::SurfaceCapabilitiesKHR,
    formats: Vec<ash::vk::SurfaceFormatKHR>,
    present_modes: Vec<ash::vk::PresentModeKHR>,
}

impl SwapchainSupportDetails {
    fn query_swapchain_support(
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

    fn choose_swap_chain_format(&self) -> SurfaceFormatKHR {
        let surface_format_khr = self.formats.iter().find(|format| {
            format.format == Format::B8G8R8_SRGB
                && format.color_space.eq(&ColorSpaceKHR::SRGB_NONLINEAR)
        });

        if surface_format_khr.is_some() {
            return *surface_format_khr.unwrap();
        }

        self.formats[0]
    }

    fn choose_present_mode(&self) -> PresentModeKHR {
        let present_mode = self
            .present_modes
            .iter()
            .find(|&present_mode| *present_mode == PresentModeKHR::MAILBOX);
        if present_mode.is_some() {
            return *present_mode.unwrap();
        }

        return PresentModeKHR::FIFO;
    }

    fn choose_swap_extent(&self, buffer_width: u32, buffer_height: u32) -> Extent2D {
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

impl ConfigurationBuilder {
    pub fn create_instance(&mut self, window: &Window) -> Result<&mut ConfigurationBuilder, &str> {
        unsafe {
            self.width = Some(1920); //TODO!
            self.height = Some(1080); //TODO!

            self.vulkan_entry =
                Some(Entry::load().expect("Failed to find vulkan library on this machine"));
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

    fn create_surface(&mut self, window: &Window) -> Result<&mut ConfigurationBuilder, &str> {
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

    fn pick_physical_device(&mut self) -> Result<&mut ConfigurationBuilder, &str> {
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

        println!("{:?}", queue_family_indices);
        queue_family_indices.is_complete() && extensions_enabled && adequate_swapchain
    }

    fn check_device_extension_support(&mut self, physical_device: &PhysicalDevice) -> bool {
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
                    println!("{:?}, {:?}", extension, device_extension_properties);
                }
            }
        }

        if flag {
            self.device_extensions.push(KHR_SWAPCHAIN_NAME.as_ptr());
        }
        flag
    }

    fn check_validation_layer_support(&self) -> Result<bool, &str> {
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

    fn create_logical_device(&mut self) -> Result<&mut ConfigurationBuilder, &str> {
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

    fn find_device_queue(&mut self, queue_family_index: u32) -> Option<Queue> {
        unsafe {
            Some(
                self.logical_device
                    .as_ref()
                    .unwrap()
                    .get_device_queue(queue_family_index, 0),
            )
        }
    }

    fn create_swap_chain(&mut self) -> Result<&mut ConfigurationBuilder, &str> {
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
                .choose_swap_extent(self.width.unwrap(), self.height.unwrap()),
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

    fn create_swapchain_image_views(&mut self) -> Result<&mut ConfigurationBuilder, &str> {
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

    fn create_shader_module<P: AsRef<Path> + std::fmt::Debug + ToString>(
        &mut self,
        path: P,
    ) -> Result<ShaderModule, &str> {
        let device = self.logical_device.as_ref().unwrap();

        let shader_binding = utils::io::read_file(&path).unwrap();
        let mut shader_as_byte_arr = Cursor::new(&shader_binding);
        let shader_spv: Vec<u32> = read_spv(&mut shader_as_byte_arr).expect("Failed to convert shader shader to spv");

        let shader_spv_c_info = ShaderModuleCreateInfo::default().code(&shader_spv);

        unsafe {
            let shader_module = device
                .create_shader_module(&shader_spv_c_info, None);

            match shader_module { 
                Ok(module) => Ok(module),
                Err(_) => {
                    error!("Failed to create shader module with path {:?}", path);
                    Err("Failed to create shader module")
                }
            }
        }
    }
   
    fn create_graphics_pipeline(&mut self) -> Result<&mut ConfigurationBuilder, &str> {
       let fragment_shader_module = self.create_shader_module(Path::new("/assets/fragment.spv").to_str().unwrap()).unwrap();
       let vertex_shader_module = self.create_shader_module(Path::new("/assets/vertex.spv").to_str().unwrap()).unwrap();
        
        let bytes = "main".as_bytes();
        let name_main = match CStr::from_bytes_with_nul(bytes) {
            Ok(bytes) => { bytes },
            Err(_) => {
                error!("Failed to parse main name");
                return Err("Failed to parse main name as bytes!");
            }
        };

       let frag_shader_create_info = PipelineShaderStageCreateInfo::default().module(fragment_shader_module)
           .stage(ShaderStageFlags::FRAGMENT).name(name_main);


        let vert_shader_create_info = PipelineShaderStageCreateInfo::default().module(module)
            .stage(ShaderStageFlags::VERTEX).name(name_main)

        let pipeline_shader_create_infos = vec![vert_shader_create_info, frag_shader_create_info];

        let dynamic_states = vec![DynamicState::VIEWPORT, DynamicState::SCISSOR];
    
        let input_assembly_create_info = PipelineInputAssemblyStateCreateInfo::default().topology(PrimitiveTopology::TRIANGLE_LIST).primitive_restart_enable(false);;

        self.viewports = vec![Viewport::default().x(0.0).y(0.0).width(self.extent.unwrap().height as f32).height(self.extent.unwrap().height as f32).min_depth(0.0).max_depth(1.0)];

        self.scissors = vec![Rect2D::default().offset(Offset2D::default().x(0).y(0)).extent(self.extent.unwrap())];
        
        let pipeline_dynamic_states_create_info= PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

        let viewport_state = PipelineViewportStateCreateInfo::default().viewports(&self.viewports).scissors(&self.scissors);

        let rasterizer_create_info = PipelineRasterizationStateCreateInfo::default().depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(CullModeFlags::BACK)
            .front_face(FrontFace::CLOCKWISE)
            .depth_bias_enable(false)
            .depth_bias_constant_factor(0.0)
            .depth_bias_clamp(0.0)
            .depth_bias_slope_factor(0.0);

        let pipeline_multisample_state_create_info = PipelineMultisampleStateCreateInfo::default().sample_shading_enable(false).rasterization_samples(SampleCountFlags::TYPE_1)
            .min_sample_shading(1.0)
            .alpha_to_coverage_enable(false)
            .alpha_to_one_enable(false);
   
        let pipeline_color_blend_attachment_state = vec![PipelineColorBlendAttachmentState::default()
            .color_write_mask(ColorComponentFlags::RGBA) // TODO: CHECK IF THIS WORKS
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
                .blend_constants([0.0,0.0,0.0,0.0]); // OPTIONAL

        let pipeline_layout_create_info =  PipelineLayoutCreateInfo::default();
        unsafe {
        let pipeline_layout = self.logical_device.unwrap().create_pipeline_layout(&pipeline_layout_create_info, None).unwrap();
        }

        Ok(self)

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
            }
        }
        0
    }

    pub fn build(&self) -> Result<Configuration, &str> {
        Ok(Configuration {
            vulkan_entry: self.vulkan_entry.clone().unwrap(),
            instance: self.instance.clone().unwrap(),
            debug_instance: self.debug_instance.clone().unwrap(),
            debug_messenger: self.debug_messenger.unwrap(),
        })
    }
}

impl Drop for ConfigurationBuilder {
    fn drop(&mut self) {
        //      unsafe { self.instance.as_ref().unwrap().destroy_instance(None); }
    }
}

impl Drop for Configuration {
    fn drop(&mut self) {
        unsafe {
            self.instance.destroy_instance(None);
        };
    }
}
