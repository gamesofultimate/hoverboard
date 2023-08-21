#![cfg(target_arch = "wasm32")]

use engine::application::components::{
  AnimationComponent, InputComponent, PhysicsComponent, TransformComponent,
};

use crate::shared::input::PlayerInput;
use engine::application::input::DefaultInput;
use engine::application::scene::Scene;
use engine::systems::{
  input::{InputsReader, CanvasController}, physics::PhysicsController, Backpack, Initializable, Inventory, System,
};

use engine::utils::units::Kph;

use nalgebra::Vector3;
// you will need a PlayerMovementSystem which uses the values from your PlayerMovementComponent and updates
// the PhysicsSystem (comes from the engine through the Backpack concept, IIRC)

const PLAYER_MAX_VELOCITY: f32 = 10.00;
const PLAYER_ACCELERATION: f32 = 5.00;
const MAX_JUMPS: u32 = 2;
const JUMP_VELOCITY: f32 = 2.0;
const ROTATION_SPEED: f32 = 10.0;

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
  fn run(&mut self, scene: &mut Scene, backpack: &mut Backpack) {
    // let delta = backpack.get::<Time>().unwrap();

    let input = match self.inputs.receive() {
      Some(input) => {
        self.capture_mouse(&input);
        input
      },
      None => return,
    };

    for (_, (_, mut physics, transform)) in
      scene.query_mut::<(&InputComponent, &mut PhysicsComponent, &TransformComponent)>()
    {
      physics.delta_translation.x = PLAYER_ACCELERATION * input.direction_vector.x;
      physics.delta_translation.z = PLAYER_ACCELERATION * input.direction_vector.z;
      physics.delta_translation.y = PLAYER_ACCELERATION * input.direction_vector.y;
    }
  }
}

impl PlayerMovementSystem {
  fn capture_mouse(&mut self, input: &PlayerInput) {
    if input.left_click && !input.mouse_lock {
      self.canvas.capture_mouse(true);
      self.canvas.request_fullscreen(true);
    }
  }

}
