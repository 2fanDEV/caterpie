use std::{error::{self, Error}, ffi::{c_void, CStr, CString}, fmt::DebugTuple};

use ash::{ext::physical_device_drm, vk::{self, ApplicationInfo, DebugUtilsMessageSeverityFlagsEXT, DebugUtilsMessageTypeFlagsEXT, DebugUtilsMessengerCallbackDataEXT, DebugUtilsMessengerCreateInfoEXT, DebugUtilsMessengerEXT, InstanceCreateFlags, InstanceCreateInfo, PhysicalDevice, PhysicalDeviceProperties2, EXT_DEBUG_UTILS_NAME, KHR_GET_PHYSICAL_DEVICE_PROPERTIES2_NAME, KHR_PORTABILITY_ENUMERATION_NAME, KHR_PORTABILITY_SUBSET_NAME}, Entry, Instance};
use log::{error, info, warn};
use winit::{keyboard, raw_window_handle::HasDisplayHandle, window::{self, Window}};


#[allow(clippy::pedantic)]

pub struct Configuration{
    vulkan_entry : Entry,
    instance: Instance,
    debug_instance: ash::ext::debug_utils::Instance,
    debug_messenger: DebugUtilsMessengerEXT
}

impl Configuration {
    pub fn default(window: &Window) -> Configuration {
        ConfigurationBuilder::default().create_instance(window).unwrap().pick_physical_device().unwrap().build().unwrap()
    }
}

pub struct ConfigurationBuilder {
    vulkan_entry: Option<Entry>,
    instance: Option<Instance>,
    debug_instance: Option<ash::ext::debug_utils::Instance>,
    debug_messenger: Option<DebugUtilsMessengerEXT>,
}


impl  ConfigurationBuilder{
    fn default() -> Self {
        Self { vulkan_entry: Default::default(), instance: Default::default(), debug_instance: Default::default(), debug_messenger: Default::default()}
    }

    pub fn create_instance(&mut self, window: &Window) -> Result<&ConfigurationBuilder, &str> {
        unsafe {
        self.vulkan_entry = Some(Entry::load().expect("Failed to find vulkan library on this machine"));
        let application_version = 1;
        let application_name = CString::new("Caterpie").unwrap();
        let engine_name = CString::new("Caterpie Engine").unwrap();
        let mut debug_messenger_create_info = DebugUtilsMessengerCreateInfoEXT::default()
            .pfn_user_callback(Some(Self::debug_callback))
            .message_severity(DebugUtilsMessageSeverityFlagsEXT::VERBOSE | DebugUtilsMessageSeverityFlagsEXT::WARNING | DebugUtilsMessageSeverityFlagsEXT::ERROR)
            .message_type(DebugUtilsMessageTypeFlagsEXT::GENERAL | DebugUtilsMessageTypeFlagsEXT::VALIDATION | DebugUtilsMessageTypeFlagsEXT::PERFORMANCE);
        let app_info = ApplicationInfo::default().application_name(&application_name)
            .engine_name(&engine_name)
            .api_version(0)
            .engine_version(1)
            .application_version(application_version);
        let entry_enumerated_instance_extensions = self.vulkan_entry.as_ref().unwrap().enumerate_instance_extension_properties(None).unwrap();
        let mut instance_extension_properties = ash_window::enumerate_required_extensions(window.display_handle().unwrap().as_raw()).unwrap().to_vec();
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
        let instance_create_info = InstanceCreateInfo::default().application_info(&app_info).flags(InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR).enabled_extension_names(&instance_extension_properties).push_next(&mut debug_messenger_create_info);
        self.instance = Some(self.vulkan_entry.as_ref().unwrap().create_instance(&instance_create_info,None).unwrap());

        self.debug_instance= Some(ash::ext::debug_utils::Instance::new(self.vulkan_entry.as_ref().unwrap(), self.instance.as_ref().unwrap()));
        self.debug_messenger = Some(self.debug_instance.as_ref().unwrap().create_debug_utils_messenger(&debug_messenger_create_info, None).unwrap());

        }
        Ok(self)
    }

    pub fn pick_physical_device(&self) -> Result<&ConfigurationBuilder, &str> {
        unsafe {
            let instance = self.instance.as_ref().unwrap();
            let physical_devices = instance.enumerate_physical_devices()
                    .expect("Failed to enumerate physical devices");

            let mut physical_device  = physical_devices.iter().find(|&p_device|  Self::is_device_suitable(*p_device));

            if physical_device.is_none()
            {
                error!("No physical device has been found, abort initialization!");
                return Err("Aborting initialization as there were no physical devices found");
            }

            Ok(self)
        }
    }

    pub fn is_device_suitable(physical_device : &PhysicalDevice) -> bool {
       true 
    }



    fn check_validation_layer_support(&self) -> Result<bool, &str> {
        let validation_layers= vec!["VK_LAYER_KHRONOS_validation"];
        unsafe { let available_layers = self.vulkan_entry.as_ref().unwrap().enumerate_instance_layer_properties().unwrap();
            for layer in validation_layers {
                for available_layer in available_layers.iter() {
                    if layer.eq(available_layer.layer_name_as_c_str().unwrap().to_str().unwrap()) {
                        return Ok(true);
                    }
                }
            }
        };
        Err("Validation Layers are not present on this machine")
    }

    unsafe extern "system" fn debug_callback(
            message_severity: DebugUtilsMessageSeverityFlagsEXT,
            message_type: DebugUtilsMessageTypeFlagsEXT,
            callback_data: *const DebugUtilsMessengerCallbackDataEXT<'_>,
            user_data : *mut c_void) -> u32 {
            unsafe{
                let p_callback_data = *callback_data;
                let message_id_name = p_callback_data.message_id_name_as_c_str().unwrap().to_string_lossy();
                let message_id_number = p_callback_data.message_id_number;
                let message = p_callback_data.message_as_c_str().unwrap().to_string_lossy();

                match message_severity {
                DebugUtilsMessageSeverityFlagsEXT::WARNING => {
                        warn!("{message_type:?} [{message_id_name} ({message_id_number})] : {message}\n");
                    },
                DebugUtilsMessageSeverityFlagsEXT::ERROR => {
                        error!("{message_type:?} [{message_id_name} ({message_id_number})] : {message}\n")

                }
                _ => {
                        info!("{message_type:?} [{message_id_name} ({message_id_number})] : {message}\n");

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
            debug_messenger: self.debug_messenger.unwrap()
        })
    }
}

impl Drop for ConfigurationBuilder {
    fn drop(&mut self) {
  //      unsafe { self.instance.as_ref().unwrap().destroy_instance(None); }
    }
}


impl  Drop for Configuration {
    fn drop(&mut self) {
        unsafe { self.instance.destroy_instance(None);
        };
    }
}
