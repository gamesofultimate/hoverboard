#![cfg(target_arch = "wasm32")]

use engine::application::components::{
  AnimationComponent, InputComponent, PhysicsComponent, TransformComponent,
};

use crate::shared::input::PlayerInput;
use engine::application::input::DefaultInput;
use engine::application::scene::Scene;
use engine::systems::{
  input::{CanvasController, InputsReader},
  physics::{PhysicsConfig, PhysicsController},
  Backpack, Initializable, Inventory, System,
};

use engine::utils::units::Kph;
use engine::utils::units::Time;

use nalgebra::Rotation3;

use nalgebra::Vector3;
// you will need a PlayerMovementSystem which uses the values from your PlayerMovementComponent and updates
// the PhysicsSystem (comes from the engine through the Backpack concept, IIRC)

const PLAYER_MAX_VELOCITY: f32 = 10.00;
const PLAYER_ACCELERATION: f32 = 500.00;
const ROTATION_SPEED: f32 = 45.0;

pub struct PlayerMovementSystem {
  inputs: InputsReader<PlayerInput>,
  physics: PhysicsController,
  canvas: CanvasController,
}

impl Initializable for PlayerMovementSystem {
  fn initialize(inventory: &Inventory) -> Self {
    let inputs = inventory.get::<InputsReader<PlayerInput>>().clone();
    let physics = inventory.get::<PhysicsController>().clone();
    let canvas = inventory.get::<CanvasController>().clone();

    Self {
      inputs,
      physics,
      canvas,
    }
  }
}

//  Get reference to
impl System for PlayerMovementSystem {
  fn attach(&mut self, _: &Scene, backpack: &mut Backpack) {
    if let Some(physics) = backpack.get_mut::<PhysicsConfig>() {
      physics.gravity = Vector3::new(0.0, 0.0, 0.0);
    }
  }

  fn run(&mut self, scene: &mut Scene, backpack: &mut Backpack) {
    let delta_time = **backpack.get::<Time>().unwrap();

    let input = match self.inputs.receive() {
      Some(input) => {
        self.capture_mouse(&input);
        input
      }
      None => return,
    };

    for (_, (_, mut physics, transform)) in scene.query_mut::<(
      &InputComponent,
      &mut PhysicsComponent,
      &mut TransformComponent,
    )>() {
      let forward_input = input.direction_vector.z;
      let right_input = -input.direction_vector.x;

      let transform_direction = transform.get_euler_direction();

      physics.delta_translation =
        transform_direction.into_inner() * PLAYER_ACCELERATION * delta_time * forward_input;

      // TODO: this needs to take into account the player's entire rotation, not just y
      physics.delta_rotation.y = ROTATION_SPEED * delta_time * right_input;
    }
  }
}

impl PlayerMovementSystem {
  fn capture_mouse(&mut self, input: &PlayerInput) {
    if input.left_click && !input.mouse_lock {
      self.canvas.capture_mouse(true);
      // self.canvas.request_fullscreen(true);
    }
  }
}
