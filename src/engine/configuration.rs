use std::ffi::CString;

use ash::{vk::{ApplicationInfo, InstanceCreateFlags, InstanceCreateInfo, KHR_PORTABILITY_ENUMERATION_NAME, KHR_PORTABILITY_SUBSET_NAME}, Entry, Instance};
use winit::{keyboard, window::Window};

#[allow(clippy::pedantic)]

pub struct Configuration<'a>{ 
    window: &'a Window,
    vulkan_entry : Entry,
}

impl <'a> Configuration <'a> {
    pub fn default(window: &'a Window) -> Configuration<'a> {
        ConfigurationBuilder::default(&window).create_instance().unwrap().build().unwrap()
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
        let instance_layer_properties = self.vulkan_entry.as_ref().unwrap().enumerate_instance_layer_properties().unwrap()
            .iter().map(|property| property.layer_name).collect::<Vec<* const i8>();
        let enabled_layer_names: Vec<* const i8> = vec![KHR_PORTABILITY_ENUMERATION_NAME.as_ptr()];
        let instance_create_info = InstanceCreateInfo::default().application_info(&app_info).flags(InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR)
            .enabled_layer_names(&enabled_layer_names);
        self.instance = Some(self.vulkan_entry.as_ref().unwrap().create_instance(&instance_create_info,None).unwrap()); 
        }
        Ok(self)
    }

    pub fn build(&self) -> Result<Configuration<'a>, &'a str> {
        Ok(Configuration {
            window: self.window,
            vulkan_entry: self.vulkan_entry.clone().unwrap()
        }) 
    }
}


impl <'a> Drop for Configuration<'a> {
    fn drop(&mut self) {
    }
}
