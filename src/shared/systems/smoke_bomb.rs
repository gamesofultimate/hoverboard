#![cfg(target_arch = "wasm32")]

use std::char::MAX;

use crate::shared::components::PlayerMovementComponent;

use engine::application::components::{
  AnimationComponent, InputComponent, PhysicsComponent, TagComponent, TransformComponent,
};
use engine::Entity;

use crate::shared::input::PlayerInput;
use engine::application::components::IdComponent;
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

pub struct SmokeBomb {

}
