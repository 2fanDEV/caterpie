use core::time;
use std::process::exit;
use std::{os::unix::thread, thread::sleep};

use log::debug;
use winit::application::ApplicationHandler;
use winit::{
    dpi::PhysicalSize,
    event::{self, KeyEvent},
    window::{Window, WindowAttributes},
};

use crate::engine::Engine;

#[derive(Default)]
pub struct App {
    request_redraw: bool,
    window: Option<Window>,
    engine: Option<Engine>,
}

const POLL_SLEEP_TIME: std::time::Duration = time::Duration::from_millis(10);

impl ApplicationHandler for App {
    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.window.as_ref().unwrap().request_redraw();
        match event_loop.control_flow() {
            winit::event_loop::ControlFlow::Poll => {
                sleep(POLL_SLEEP_TIME);
            }
            _ => todo!(),
        }
    }

    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes = WindowAttributes::default()
            .with_inner_size(PhysicalSize::new(1920, 1080))
            .with_decorations(true);
        self.window = Some(event_loop.create_window(window_attributes).unwrap());
        self.engine = Some(Engine::init(&self.window.as_ref().unwrap()).unwrap());
        debug!("App resumed");
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match &mut self.engine {
            Some(engine) => {
                engine.draw_frame();
                match event {
                    event::WindowEvent::Destroyed => {
                        engine.destroy();
                    }
                    event::WindowEvent::CloseRequested => {
                        engine.destroy();
                        exit(0);
                    }
                    event::WindowEvent::Resized(size) => {
                        engine.window_resized(size);
                    }
                    event::WindowEvent::KeyboardInput {
                        device_id,
                        event,
                        is_synthetic,
                    } => match event {
                        KeyEvent {
                            physical_key,
                            logical_key,
                            text,
                            location,
                            state,
                            repeat,
                            ..
                        } => if logical_key.eq("e") {},
                    },
                    _ => {}
                }
            }
            None => {}
        }
    }
}
