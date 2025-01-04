use winit::{application::ApplicationHandler, dpi::LogicalSize, window::{self, Window, WindowAttributes}};

use crate::engine::engine::Engine;

#[derive(Default)]
pub struct App { 
    window: Option<Window>
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) { 
        let window_attributes = WindowAttributes::default().with_inner_size(LogicalSize::new(1920,1080));
        self.window = Some(event_loop.create_window(window_attributes).unwrap());
        let engine = Engine::init(&self.window.as_ref().unwrap()).unwrap();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        
    }
}
