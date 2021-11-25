use std::collections::HashMap;
use std::time::Instant;

use glium::glutin::event::{ElementState, KeyboardInput, MouseButton, VirtualKeyCode};
use glium::Display;
use nalgebra_glm::{radians, vec1, vec3, Vec3};

pub struct GameState {
    dragging: bool,
    held_keys: HashMap<VirtualKeyCode, bool>,
    pub pos: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    last_timestamp: Instant,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            dragging: false,
            held_keys: [
                VirtualKeyCode::W,
                VirtualKeyCode::S,
                VirtualKeyCode::A,
                VirtualKeyCode::D,
                VirtualKeyCode::Space,
                VirtualKeyCode::LControl,
                VirtualKeyCode::LShift,
            ]
            .into_iter()
            .map(|code| (code, false))
            .collect(),
            pos: vec3(-4875.0, -1237.0, 140.0),
            yaw: std::f32::consts::PI,
            pitch: 0.0,
            last_timestamp: Instant::now(),
        }
    }

    pub fn handle_mouse_motion(&mut self, delta: (f64, f64)) {
        if self.dragging {
            self.yaw = (self.yaw + 0.01 * delta.0 as f32).rem_euclid(std::f32::consts::TAU);
            self.pitch = (self.pitch - 0.01 * delta.1 as f32)
                .clamp(radians(&vec1(-89.0)).x, radians(&vec1(89.0)).x)
        }
    }

    pub fn handle_mouse_input(
        &mut self,
        display: &Display,
        button: MouseButton,
        state: ElementState,
    ) {
        if button == MouseButton::Left {
            self.dragging = state == ElementState::Pressed;
            display
                .gl_window()
                .window()
                .set_cursor_grab(self.dragging)
                .unwrap();
        }
        if button == MouseButton::Right && state == ElementState::Pressed {
            println!("pos: {:?}", self.pos);
        }
    }

    pub fn handle_keyboard_input(&mut self, input: KeyboardInput) {
        if let Some(code) = input.virtual_keycode {
            if let Some(flag) = self.held_keys.get_mut(&code) {
                *flag = input.state == ElementState::Pressed;
            }
        }
    }

    pub fn step(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_timestamp).as_secs_f32();
        self.last_timestamp = now;

        let forward = vec3(self.yaw.cos(), -self.yaw.sin(), 0.0);
        let right = vec3(-self.yaw.sin(), -self.yaw.cos(), 0.0);
        let up = vec3(0.0, 0.0, 1.0);
        let delta_pos = if self.held_keys[&VirtualKeyCode::LShift] {
            1000.0
        } else {
            100.0
        } * dt;
        if self.held_keys[&VirtualKeyCode::W] {
            self.pos += delta_pos * forward;
        }
        if self.held_keys[&VirtualKeyCode::S] {
            self.pos -= delta_pos * forward;
        }
        if self.held_keys[&VirtualKeyCode::A] {
            self.pos -= delta_pos * right;
        }
        if self.held_keys[&VirtualKeyCode::D] {
            self.pos += delta_pos * right;
        }
        if self.held_keys[&VirtualKeyCode::Space] {
            self.pos += delta_pos * up;
        }
        if self.held_keys[&VirtualKeyCode::LControl] {
            self.pos -= delta_pos * up;
        }
    }
}
