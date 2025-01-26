use log::debug;
use winit::{application::ApplicationHandler, dpi::{LogicalSize, PhysicalSize, Size}, event::{self, KeyEvent}, window::{self, Window, WindowAttributes}};

use crate::engine::engine::Engine;

#[derive(Default)]
pub struct App{
    window: Option<Window>,
    engine: Option<Engine>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes = WindowAttributes::default().with_inner_size(PhysicalSize::new(1920, 1080));
        self.window = Some(event_loop.create_window(window_attributes).unwrap());
        self.engine = Some(Engine::init(&self.window.as_ref().unwrap()).unwrap());
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match &mut self.engine {
            Some(engine) => {
                println!("draw frame");
                engine.draw_frame();
                match event {
                    event::WindowEvent::Resized(size) => {
                        engine.window_resized();

                        debug!("WINDOW RESIZED EVENT");
                    },
                    event::WindowEvent::KeyboardInput { device_id, event, is_synthetic } => {
                       match event {
                        KeyEvent { physical_key, logical_key, text, location, state, repeat, .. } => {
                            if logical_key.eq("e") {
                            }
                        }
                       } 
                    }
                    _ => {}
                }
            },
            None => {},
        }
    }
}



