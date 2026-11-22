//! GUI mode — Phase 2: window, fly camera, and the first rendered world.
//!
//! Input model: the cursor is grabbed while the window is focused; mouse
//! deltas steer the camera (recenter-to-middle strategy), WASD/Space/Shift
//! fly the camera. Losing focus releases the cursor and clears keys so
//! alt-tab never leaves the camera drifting.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use glam::{Mat4, Vec3};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowId};

use crate::camera::{Camera, MoveInput, SPAWN, SPAWN_PITCH, SPAWN_YAW};
use crate::renderer::Gpu;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = App::default();
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[derive(Default)]
struct App {
    gpu: Option<Gpu>,
    camera: Option<Camera>,
    keys: HashSet<PhysicalKey>,
    last_frame: Option<Instant>,
    mouse_captured: bool,
    /// The next `CursorMoved` is our own `set_cursor_position` warp — skip it.
    skip_warp_event: bool,
}

impl App {
    fn move_input(&self) -> MoveInput {
        let pressed = |code: KeyCode| self.keys.contains(&PhysicalKey::Code(code));
        MoveInput {
            forward: (pressed(KeyCode::KeyW) as i32 - pressed(KeyCode::KeyS) as i32) as f32,
            strafe: (pressed(KeyCode::KeyD) as i32 - pressed(KeyCode::KeyA) as i32) as f32,
            vertical: (pressed(KeyCode::Space) as i32 - pressed(KeyCode::ShiftLeft) as i32) as f32,
        }
    }

    fn grab_cursor(&mut self, window: &Window) {
        // Locked on platforms that support it, Confined as fallback.
        let grabbed = window
            .set_cursor_grab(CursorGrabMode::Locked)
            .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined))
            .is_ok();
        window.set_cursor_visible(!grabbed);
        self.mouse_captured = grabbed;
        self.skip_warp_event = false;
    }

    fn release_cursor(&mut self, window: &Window) {
        let _ = window.set_cursor_grab(CursorGrabMode::None);
        window.set_cursor_visible(true);
        self.mouse_captured = false;
    }

    /// One frame: integrate movement, then draw. Borrows against `self.gpu`
    /// and `self.camera` are taken sequentially, never overlapping.
    fn frame(&mut self) {
        let view_proj = match (&self.camera, self.gpu.as_ref()) {
            (Some(camera), Some(gpu)) => Some(camera.view_proj(gpu.aspect())),
            _ => None,
        };
        let input = self.move_input();
        let dt = self
            .last_frame
            .take()
            .map(|t| t.elapsed().as_secs_f32())
            .unwrap_or(1.0 / 60.0);
        self.last_frame = Some(Instant::now());
        if let Some(camera) = self.camera.as_mut() {
            camera.apply_move(dt.min(0.1), input);
        }
        if let Some(gpu) = self.gpu.as_mut() {
            gpu.render(view_proj.unwrap_or(Mat4::IDENTITY));
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, elwt: &ActiveEventLoop) {
        if self.gpu.is_some() {
            return;
        }
        let window = Arc::new(
            elwt.create_window(
                Window::default_attributes()
                    .with_title("MELON — dev window (Phase 2)")
                    .with_inner_size(LogicalSize::new(1280.0, 720.0)),
            )
            .expect("create window"),
        );
        let gpu = pollster::block_on(Gpu::new(window)).expect("init wgpu");

        self.camera = Some(Camera::new(Vec3::from(SPAWN), SPAWN_YAW, SPAWN_PITCH));
        self.grab_cursor(&gpu.window);
        self.last_frame = Some(Instant::now());
        self.gpu = Some(gpu);
        println!("window up — world rendered; WASD/Space/Shift to fly, ESC to quit");
    }

    fn window_event(&mut self, elwt: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        // Window-level events that need no GPU borrow.
        match &event {
            WindowEvent::CloseRequested => {
                elwt.exit();
                return;
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    if event.logical_key == Key::Named(NamedKey::Escape) {
                        elwt.exit();
                        return;
                    }
                    self.keys.insert(event.physical_key);
                } else {
                    self.keys.remove(&event.physical_key);
                }
                return;
            }
            WindowEvent::Focused(focused) => {
                let Some(window) = self.gpu.as_ref().map(|g| Arc::clone(&g.window)) else {
                    return;
                };
                self.keys.clear();
                if *focused {
                    self.grab_cursor(&window);
                } else {
                    self.release_cursor(&window);
                }
                return;
            }
            _ => {}
        }

        let Some(gpu) = self.gpu.as_mut() else { return };
        if window_id != gpu.window.id() {
            return;
        }

        match event {
            WindowEvent::Resized(size) => gpu.resize(size),
            WindowEvent::CursorMoved { position, .. } => {
                if !self.mouse_captured || self.skip_warp_event {
                    self.skip_warp_event = false;
                    return;
                }
                let size = gpu.window.inner_size();
                let center =
                    PhysicalPosition::new(size.width as f64 / 2.0, size.height as f64 / 2.0);
                let dx = position.x - center.x;
                let dy = position.y - center.y;
                if dx.abs() < 0.5 && dy.abs() < 0.5 {
                    return; // already centered — nothing to feed the camera
                }
                let Some(camera) = self.camera.as_mut() else {
                    return;
                };
                camera.look(dx as f32, dy as f32);
                // Warp back to the middle; skip the event that warp fires.
                self.skip_warp_event = true;
                let _ = gpu.window.set_cursor_position(center);
            }
            WindowEvent::RedrawRequested => self.frame(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _elwt: &ActiveEventLoop) {
        if let Some(gpu) = self.gpu.as_ref() {
            gpu.window.request_redraw();
        }
    }
}
