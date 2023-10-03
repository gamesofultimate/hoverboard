#![cfg(target_arch = "wasm32")]
use serde::{Deserialize, Serialize};
use crate::shared::input::PlayerInput;

use engine::{
  application::{
    components::LightComponent,
    // input::DefaultInput,
    scene::{Scene, UnpackEntity, IdComponent},
  },
  systems::{
    input::{CanvasController, InputsReader},
    network::{ServerReceiver, ServerSender},
    Backpack, Initializable, Inventory, System,
  },
  utils::units::{Radian, Time},
};
use nalgebra::Vector3;

// use crate::shared::player_controller::PlayerController;
use crate::shared::systems::player_movement::PlayerMovementSystem;

pub struct WorldSystem {
  inputs: InputsReader<PlayerInput>,
  // canvas: CanvasController,
  // player_controller: PlayerController,
  player_movement: PlayerMovementSystem,
}

impl Initializable for WorldSystem {
  fn initialize(inventory: &Inventory) -> Self {
    let inputs = inventory.get::<InputsReader<PlayerInput>>().clone();
    // let canvas = inventory.get::<CanvasController>().clone();
    // let player_controller = PlayerController::new();
    let player_movement = PlayerMovementSystem::initialize(inventory);

    Self {
      inputs,
      // canvas,
      player_movement,
    }
  }
}

impl WorldSystem {
  // fn capture_mouse(&mut self, input: &PlayerInput) {
  //   if input.left_click && !input.mouse_lock {
  //     self.canvas.capture_mouse(true);
  //     self.canvas.request_fullscreen(true);
  //   }
  // }

  fn control_player(&mut self, scene: &mut Scene, input: &PlayerInput, delta: f32) {
    // if self.player_controller.is_initialized() == false {
    //   for (entity, id) in &mut scene.query::<&IdComponent>() {
    //     if !id.is_self {
    //       continue;
    //     }
    //     self.player_controller.initialize(&scene, entity, id.id);
    //   }
    // }

    // self.player_controller.input(&input);
    // self.player_controller.update(&scene, delta);
  }
}

impl System for WorldSystem {
  fn run(&mut self, scene: &mut Scene, backpack: &mut Backpack) {
    let delta = backpack.get::<Time>().unwrap();

    // if let Some(input) = self.inputs.receive() {
    //   self.capture_mouse(&input);
    //   // self.control_player(scene, &input, **delta);
    // }

    self.player_movement.run(scene, backpack);

    for (_, light) in scene.query_mut::<&mut LightComponent>() {
      if let LightComponent::Directional {
        inclination,
        azimuth,
        ..
      } = light
      {
        //*inclination += Radian::from_degree(0.001);
        //*azimuth += Radian::from_degree(0.1);
      }
    }
    /*
    let mut directional_radiance = Vector3::zeros();

    let mut sky_azimuth = Radian::from_degree(0.0);
    let mut sky_inclination = Radian::from_degree(0.0);

    // Set the lighting orientation
    // This particular system sets it so that it's always moving
    // so we can showcase the realtime lighting system
    for (_, light) in scene.query_mut::<&LightComponent>() {
      if let LightComponent::Directional { radiance, azimuth, inclination, .. } = light {
        sky_azimuth = *azimuth;
        sky_inclination = *inclination;
        directional_radiance = radiance.clone();
      }
    }
    // Set the lighting orientation
    */
  }
}
