#![cfg(target_arch = "wasm32")]

use engine::application::devices::{Devices, KeyboardKey, MouseButton, MouseEvent, WindowEvent};
use engine::systems::input::Input;
use nalgebra::{Vector2, Vector3};

#[derive(Clone, Debug)]
pub struct PlayerInput {
  pub direction_vector: Vector3<f32>,
  pub mouse_delta: Vector2<f32>,
  pub mouse_position: Vector2<f32>,
  pub mouse_lock: bool,
  pub is_fullscreen: bool,
  pub focused: bool,
  pub left_click: bool,
  pub right_click: bool,
  pub canvas: (u32, u32),
  pub pixel_ratio: f32,
  pub keyboard: Vec<KeyboardKey>,
}

impl Default for PlayerInput {
  fn default() -> Self {
    Self::new(1920, 1080)
  }
}

impl Input for PlayerInput {
  fn reset(&mut self) {
    self.direction_vector = Vector3::zeros();
    self.mouse_delta = Vector2::zeros();
    self.mouse_position = Vector2::zeros();
  }

  fn from_devices(&mut self, device: &mut Devices) {
    self.focused = device.window.focus;

    self.canvas = device.window.canvas_size;
    self.pixel_ratio = device.window.pixel_ratio;

    for event in device.mouse.iter_buttons() {
      match event {
        MouseButton::Primary => {
          self.left_click = true;
        }
        MouseButton::Secondary => {
          self.right_click = true;
        }
        _ => {}
      }
    }

    //log::info!("device.mouse {:?}", &device.mouse);
    for event in device.mouse.iter_events() {
      match event {
        MouseEvent::Motion { x, y, dx, dy } => {
          self.mouse_delta.x = dx;
          self.mouse_delta.y = dy;
        }

        MouseEvent::Wheel { dx, dy } => {
          self.mouse_delta.x = dx;
          self.mouse_delta.y = dy;
        }
        _ => {}
      }
    }

    for key in device.iter_keyboard() {
      match key {
        KeyboardKey::D | KeyboardKey::Right => self.direction_vector.x = 1.0,
        KeyboardKey::A | KeyboardKey::Left => self.direction_vector.x = -1.0,
        KeyboardKey::W | KeyboardKey::Up => self.direction_vector.z = 1.0,
        KeyboardKey::S | KeyboardKey::Down => self.direction_vector.z = -1.0,
        _ => {}
      }
    }

    self.keyboard = device.keyboard.clone();

    for (_, gamepad) in &device.gamepads {
      const MIN_EPSILON: f32 = 0.0 - 0.02;
      const MAX_EPSILON: f32 = 0.0 + 0.02;

      if gamepad.left_joystick.x > MAX_EPSILON || gamepad.left_joystick.x < MIN_EPSILON {
        self.direction_vector.x = gamepad.left_joystick.x;
      }
      if gamepad.left_joystick.y > MAX_EPSILON || gamepad.left_joystick.y < MIN_EPSILON {
        self.direction_vector.z = -gamepad.left_joystick.y;
      }
      if gamepad.left_joystick.x > MAX_EPSILON || gamepad.left_joystick.x < MIN_EPSILON {
        self.mouse_delta.x = gamepad.right_joystick.x;
      }
      if gamepad.left_joystick.y > MAX_EPSILON || gamepad.left_joystick.y < MIN_EPSILON {
        self.mouse_delta.y = gamepad.right_joystick.y;
      }
    }

    for event in device.window.iter_events() {
      match event {
        WindowEvent::CaptureMouse => self.mouse_lock = true,
        WindowEvent::ReleaseMouse => self.mouse_lock = false,
        WindowEvent::RequestFullscreen => self.is_fullscreen = true,
        WindowEvent::ReleaseFullscreen => self.is_fullscreen = false,
      };
    }
  }
}

impl PlayerInput {
  pub fn new(width: u32, height: u32) -> Self {
    Self {
      direction_vector: Vector3::zeros(),
      mouse_delta: Vector2::zeros(),
      mouse_position: Vector2::zeros(),
      mouse_lock: false,
      is_fullscreen: false,
      focused: false,
      left_click: false,
      right_click: false,
      canvas: (0, 0),
      pixel_ratio: 1.0,
      keyboard: Vec::new(),
    }
  }
}
