use app::App;
use engine::{configuration::Configuration, engine::Engine};
use winit::event_loop::EventLoop;

mod app;
mod engine;

fn main() {
    let mut app = App::default();
    let event_loop = EventLoop::new().unwrap();
    event_loop.run_app(&mut app).unwrap();
    println!("Hello, world!");
}
