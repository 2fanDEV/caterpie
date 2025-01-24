use winit::window::Window;

use super::Configuration;



#[derive(Default)]
pub struct Engine{
    configuration: Option<Configuration>
}


impl  Engine {

    pub fn init(window: &Window) -> Result<Engine,  &str> {
        let configuration : Configuration = Configuration::default(window);
        Ok(Self{configuration: Some(configuration)})
    }




}
