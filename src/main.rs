use app::App;
use utils::io::*;
use engine::{configuration::Configuration, engine::Engine};
use log::{info, LevelFilter};
use winit::event_loop::EventLoop;

mod app;
mod engine;
mod utils;

fn main() {
    let mut app = App::default();
    let event_loop = EventLoop::new().unwrap();
    env_logger::builder().filter_level(LevelFilter::Debug).try_init();
    println!("TEST");
    info!("test");
    event_loop.run_app(&mut app).unwrap();
    println!("Hello, world!");
}
