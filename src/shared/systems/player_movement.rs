#![cfg(target_arch = "wasm32")]

use std::char::MAX;

use crate::shared::components::PlayerMovementComponent;

use engine::application::{
  components::{AnimationComponent, InputComponent, PhysicsComponent},
  scene::{component_registry::Access, IdComponent, TagComponent, TransformComponent},
};
use engine::Entity;
use rapier3d::prelude::*;

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

const PLAYER_MAX_VELOCITY: f32 = 2000.0;
const PLAYER_ACCELERATION: f32 = 200.0;
const ROTATION_SPEED: f32 = 110.0;
const MIN_HEIGHT_FROM_SURFACE: f32 = 0.02;
const MAX_HEIGHT_FROM_SURFACE: f32 = 0.35;
const HEIGHT_FROM_SURFACE_SPEED: f32 = 6.0;
const PLAYER_DECELERATION: f32 = 150.0;

pub struct PlayerMovementSystem {
  inputs: InputsReader<PlayerInput>,
  physics_controller: PhysicsController,
  canvas: CanvasController,
  running_time: f32,
  initialized: bool,
  current_velocity: f32,
}

impl Initializable for PlayerMovementSystem {
  fn initialize(inventory: &Inventory) -> Self {
    let inputs = inventory.get::<InputsReader<PlayerInput>>().clone();
    let physics_controller: PhysicsController = inventory.get::<PhysicsController>().clone();
    let canvas = inventory.get::<CanvasController>().clone();

    Self {
      inputs,
      physics_controller,
      canvas,
      running_time: 0.0,
      initialized: false,
      current_velocity: 0.0,
    }
  }
}

impl System for PlayerMovementSystem {
  fn provide(&mut self, inventory: &Inventory) {
    PlayerMovementComponent::register();
  }

  fn attach(&mut self, scene: &mut Scene, backpack: &mut Backpack) {
    if let Some(physics) = backpack.get_mut::<PhysicsConfig>() {
      physics.gravity = Vector3::new(0.0, 0.0, 0.0);
    }
  }

  fn run(&mut self, scene: &mut Scene, backpack: &mut Backpack) {
    let delta_time = **backpack.get::<Time>().unwrap();

    let input = self.inputs.read();

    self.handle_input(scene, input, delta_time);
    self.handle_hover(scene, delta_time);

    self.running_time += delta_time;
  }
}

impl PlayerMovementSystem {
  fn accelerate(&mut self, forward_input: f32, delta_time: f32) {
    if forward_input != 0.0 {
      if self.current_velocity >= PLAYER_MAX_VELOCITY {
        self.current_velocity = PLAYER_MAX_VELOCITY;
      } else if self.current_velocity <= -PLAYER_MAX_VELOCITY {
        self.current_velocity = -PLAYER_MAX_VELOCITY;
      } else {
        self.current_velocity += PLAYER_ACCELERATION * delta_time * forward_input;
      }
    } else {
      self.decelerate(delta_time);
    }
  }

  fn capture_mouse(&mut self, input: &PlayerInput) {
    if input.left_click && !input.mouse_lock {
      self.canvas.capture_mouse(true);
      // self.canvas.request_fullscreen(true);
    }
  }

  fn decelerate(&mut self, delta_time: f32) {
    if self.current_velocity > 0.0 {
      self.current_velocity -= PLAYER_DECELERATION * delta_time;
      if self.current_velocity < 0.0 {
        self.current_velocity = 0.0;
      }
    } else if self.current_velocity < 0.0 {
      self.current_velocity += PLAYER_DECELERATION * delta_time;
      if self.current_velocity > 0.0 {
        self.current_velocity = 0.0;
      }
    }
  }

  fn handle_input(&mut self, scene: &mut Scene, input: PlayerInput, delta_time: f32) {
    for (_, (player_component, mut physics, transform)) in scene.query_mut::<(
      &PlayerMovementComponent,
      &mut PhysicsComponent,
      &mut TransformComponent,
    )>() {
      let forward_input = input.direction_vector.z;
      let right_input = -input.direction_vector.x;

      let transform_direction = transform.get_euler_direction();

      self.accelerate(forward_input, delta_time);
      self.physics_controller.set_linvel(
        physics,
        transform_direction.into_inner() * self.current_velocity * delta_time * forward_input,
      );

      // TODO: this needs to take into account the player's entire rotation, not just y
      let player_up = player_component.down_vector;
      self.physics_controller.set_angvel(
        physics,
        player_up * ROTATION_SPEED * delta_time * right_input,
      );
    }
  }

  fn handle_hover(&mut self, scene: &mut Scene, delta_time: f32) {
    for (entity, (mut player_component, mut physics, transform)) in scene.query_mut::<(
      &mut PlayerMovementComponent,
      &mut PhysicsComponent,
      &mut TransformComponent,
    )>() {
      // I thought maybe cancelling "hover" effect would help with the jittering, but it doesn't seem to?
      if self.current_velocity < 0.001 && self.current_velocity > -0.001 {
        return;
      }

      let height_delta = MAX_HEIGHT_FROM_SURFACE - MIN_HEIGHT_FROM_SURFACE;

      let player_up = -player_component.down_vector;

      let ray = Ray::new(transform.translation.into(), -player_up);
      let toi = 1.00;
      let solid = true;

      if let Some(rigidbody_handle) = self
        .physics_controller
        .get_rigid_body(&physics.joint.body.id)
      {
        let filter = QueryFilter::default();
        let filter = filter.exclude_rigid_body(rigidbody_handle);

        if let Some((_, collider, intersection)) =
          self.physics_controller.raycast(&ray, toi, solid, filter)
        {
          log::info!("intersection: {:?}", intersection);
          player_component.down_vector = -intersection.normal;
        }
      }

      let player_up = -player_component.down_vector;

      let old_linvel = self.physics_controller.linvel(physics);

      self.physics_controller.set_linvel(
        physics,
        old_linvel
          + player_up
            * (f32::sin(self.running_time * HEIGHT_FROM_SURFACE_SPEED) * height_delta
              + MIN_HEIGHT_FROM_SURFACE),
      );
    }
  }
}
