use std::{error::Error, ffi::CString};

use ash::{vk::{self, ApplicationInfo, InstanceCreateFlags, InstanceCreateInfo, PhysicalDeviceProperties2, KHR_GET_PHYSICAL_DEVICE_PROPERTIES2_NAME, KHR_PORTABILITY_ENUMERATION_NAME, KHR_PORTABILITY_SUBSET_NAME}, Entry, Instance};
use winit::{keyboard, raw_window_handle::HasDisplayHandle, window::Window};

use crate::utils::{log::log};

#[allow(clippy::pedantic)]

pub struct Configuration<'a>{
    window: &'a Window,
    vulkan_entry : Entry,
    instance: Instance,
}

impl <'a> Configuration <'a> {
    pub fn default(window: &'a Window) -> Configuration<'a> {
        ConfigurationBuilder::default(window).create_instance().unwrap().build().unwrap()
    }
}

pub struct ConfigurationBuilder<'a> {
    window: &'a Window,
    vulkan_entry: Option<Entry>,
    instance: Option<Instance>,
}


impl <'a> ConfigurationBuilder<'a>{
    fn default(window: &'a Window) -> Self {
        Self { vulkan_entry: Default::default(), instance: Default::default(), window}
    }

    pub fn create_instance(&mut self) -> Result<&ConfigurationBuilder<'a>, &'a str> {
        unsafe {
        self.vulkan_entry = Some(Entry::load().expect("Failed to find vulkan library on this machine"));
        let application_version = 1;
        let application_name = CString::new("Caterpie").unwrap();
        let engine_name = CString::new("Caterpie Engine").unwrap();
        let app_info = ApplicationInfo::default().application_name(&application_name)
            .engine_name(&engine_name)
            .api_version(0)
            .engine_version(1)
            .application_version(application_version);
        let entry_enumerated_instance_extensions = self.vulkan_entry.as_ref().unwrap().enumerate_instance_extension_properties(None).unwrap();
        let mut instance_extension_properties = ash_window::enumerate_required_extensions(self.window.display_handle().unwrap().as_raw()).unwrap().to_vec();
        instance_extension_properties.push(KHR_PORTABILITY_ENUMERATION_NAME.as_ptr());
        instance_extension_properties.push(KHR_GET_PHYSICAL_DEVICE_PROPERTIES2_NAME.as_ptr());

        for extension in entry_enumerated_instance_extensions {
            instance_extension_properties.push(extension.extension_name.as_ptr());
        }

        match self.check_validation_layer_support() {
            Ok(_) => self.setup_debug_messenger(),
            Err(_) => log("ERROR: VALIDATION LAYERS ARE NOT PRESENT ON THIS MACHINE, PROCEEDING WITHOUT SETTING UP DEBUG MESSENGER")
        }

        let instance_create_info = InstanceCreateInfo::default().application_info(&app_info).flags(InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR).enabled_extension_names(&instance_extension_properties);
        self.instance = Some(self.vulkan_entry.as_ref().unwrap().create_instance(&instance_create_info,None).unwrap());
        }
        Ok(self)
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

    fn setup_debug_messenger(&self) {

    }

    pub fn build(&self) -> Result<Configuration<'a>, &'a str> {
        Ok(Configuration {
            window: self.window,
            vulkan_entry: self.vulkan_entry.clone().unwrap(),
            instance: self.instance.clone().unwrap()
        })
    }
}

impl <'a> Drop for ConfigurationBuilder<'a> {
    fn drop(&mut self) {
//        unsafe { self.instance.as_ref().unwrap().destroy_instance(None); }
    }
}


impl <'a> Drop for Configuration<'a> {
    fn drop(&mut self) {
        unsafe { self.instance.destroy_instance(None) };
    }
}
