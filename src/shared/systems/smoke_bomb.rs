#![cfg(target_arch = "wasm32")]
use crate::shared::components::PlayerMovementComponent;
use crate::shared::input::{Actions, PlayerInput};
use engine::application::components::IdComponent;
use engine::application::components::{
  AnimationComponent, InputComponent, ParentComponent, PhysicsComponent, TagComponent,
  TransformComponent,
};

use engine::application::input::DefaultInput;
use engine::application::scene::Scene;
use engine::systems::{
  input::{CanvasController, InputsReader},
  physics::{PhysicsConfig, PhysicsController},
  Backpack, Initializable, Inventory, System,
};

use engine::utils::units::Kph;
use engine::utils::units::Time;
use engine::Entity;
use nalgebra::Rotation3;
use nalgebra::Vector3;
use std::char::MAX;

const ACTIVE_TIME: f32 = 10.0;

pub struct SmokeBombSystem {
  timer: f32,
  inputs: InputsReader<PlayerInput>,
  is_active: bool,
}

impl initializable for SmokeBombSystem {
  fn initialize(inventory: &Inventory) -> Self {
    let inputs = inventory.get::<InputsReader<PlayerInput>>().clone();

    Self {
      timer: 0.0,
      inputs,
      is_active: false,
    }
  }
}

impl System for SmokeBombSystem {
  fn run(&mut self, scene: &mut Scene, backpack: &mut Backpack) {
    let delta_time = **backpack.get::<Time>().unwrap();

    let input = match self.inputs.receive() {
      Some(input) => input,
      None => return,
    };

    self.handle_input(scene, input, delta_time);
    if self.is_active {
      self.timer += delta_time;
    }
  }
}

impl SmokeBombSystem {
  fn handle_input(&mut self, scene: &mut Scene, input: &PlayerInput, delta_time: f32) {
    let smoke_bomb = scene.get_prefab("Smoke Bomb").unwrap().clone();
    let id = smoke_bomb.id();
    let entity = scene.create_entity(id);

    if input.actions.contains(&Actions::SmokeBomb) && !self.is_active {
      self.is_active = true;
      smoke_bomb.parent = ParentComponent::new(id);
      smoke_bomb.unpack(scene, &entity);
    }

    if self.timer >= ACTIVE_TIME {
      self.is_active = false;
      self.timer = 0.0;
      scene.remove_entity(entity);
    }
  }
}
