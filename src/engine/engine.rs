use winit::window::Window;

use super::configuration::Configuration;


#[derive(Default)]
pub struct Engine<'a>{
    configuration: Option<Configuration<'a>>
}


impl <'a> Engine <'a> {

    pub fn init(window: &'a Window) -> Result<Engine, &'a str> {
        let configuration : Configuration<'a> = Configuration::default(window);

        Ok(Self{configuration: Some(configuration)})
    }

    fn create_instance(&self) {

    }


}
