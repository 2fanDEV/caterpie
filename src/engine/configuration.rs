use std::ffi::{c_void, CStr, CString};

use ash::{
    vk::{
        ApplicationInfo, DebugUtilsMessageSeverityFlagsEXT, DebugUtilsMessageTypeFlagsEXT,
        DebugUtilsMessengerCallbackDataEXT, DebugUtilsMessengerCreateInfoEXT,
        DebugUtilsMessengerEXT, DeviceCreateInfo, DeviceQueueCreateInfo, ExtensionProperties,
        InstanceCreateFlags, InstanceCreateInfo, PhysicalDevice, PhysicalDeviceFeatures, Queue,
        QueueFlags, SurfaceKHR, EXT_DEBUG_UTILS_NAME, KHR_GET_PHYSICAL_DEVICE_PROPERTIES2_NAME,
        KHR_PORTABILITY_ENUMERATION_NAME, KHR_SWAPCHAIN_NAME,
    },
    Device, Entry, Instance,
};
use log::{error, info, warn};
use winit::{
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::Window,
};

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
            .unwrap()
            .create_surface(window)
            .unwrap()
            .pick_physical_device()
            .unwrap()
            .create_logical_device()
            .unwrap()
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
    swapchain_support_details: Option<SwapchainSupportDetails>,

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
        instance: Instance,
        surface_instance: ash::khr::surface::Instance,
        surface: SurfaceKHR,
        physical_device: PhysicalDevice,
    ) -> SwapchainSupportDetails {
        unsafe {
            let capabilities = surface_instance
                .get_physical_device_surface_capabilities(physical_device, surface)
                .unwrap();
            let formats = surface_instance
                .get_physical_device_surface_formats(physical_device, surface)
                .unwrap();
            let present_modes = surface_instance
                .get_physical_device_surface_present_modes(physical_device, surface)
                .unwrap();
            SwapchainSupportDetails {
                capabilities,
                formats,
                present_modes,
            }
        }
    }
}

impl ConfigurationBuilder {
    pub fn create_instance(&mut self, window: &Window) -> Result<&mut ConfigurationBuilder, &str> {
        unsafe {
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
                self.instance.as_ref().unwrap().clone(),
                self.surface_instance.as_ref().unwrap().clone(),
                self.surface.unwrap(),
                *physical_device,
            );
            self.swapchain_support_details = Some(swapchain_support_details.clone());
            adequate_swapchain = !swapchain_support_details.formats.is_empty() && !swapchain_support_details.present_modes.is_empty();
        }

        println!("{:?}", queue_family_indices);
        queue_family_indices.is_complete() &&  extensions_enabled && adequate_swapchain
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
