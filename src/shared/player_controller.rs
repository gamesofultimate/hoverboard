use crate::shared::weapon::great_ax::GreatAx;
use crate::shared::weapon::Weapon;

use engine::application::components::AnimationComponent;
use engine::application::components::InputComponent; // should be free range
use engine::application::components::PhysicsComponent;
use engine::application::components::TransformComponent;

use engine::application::devices::KeyboardKey;

use engine::application::physics3d::ColliderHandle;

use engine::application::scene::Entity;
use engine::application::scene::Scene;

use engine::application::input::DefaultInput;

use engine::utils::units::Kph;

use nalgebra::Vector3;
use uuid::Uuid;

const MAX_JUMPS: u32 = 2;
const JUMP_VELOCITY: f32 = 2.0;

pub struct PlayerController {
  entity: Entity,
  id: Uuid,
  weapon: Option<Box<dyn Weapon>>,

  delta_translation: Vector3<f32>,
  delta_rotation: Vector3<f32>,
  delta_velocity: Vector3<f32>,

  is_grounded: bool,
  current_jumps: u32,
  should_jump: bool,
}

impl PlayerController {
  pub fn new() -> Self {
    PlayerController {
      entity: Entity::DANGLING,
      id: Uuid::nil(),
      weapon: Some(Box::new(GreatAx::new())),
      delta_translation: Vector3::zeros(),
      delta_rotation: Vector3::zeros(),
      delta_velocity: Vector3::zeros(),
      is_grounded: false,
      current_jumps: 0,
      should_jump: false,
    }
  }

  pub fn is_initialized(&self) -> bool {
    self.id.is_nil() == false
  }

  pub fn initialize(&mut self, scene: &Scene, entity: Entity, id: Uuid) {
    self.entity = entity;
    self.id = id;

    let mut query = scene.query::<&mut InputComponent>();

    if let Some(mut free_range) = query.view().get_mut(self.entity) {
      free_range.speed = Kph::from_meters_per_second(0.3);
    }
  }

  fn reset_input(&mut self) {
    self.delta_translation = Vector3::zeros();
    self.delta_rotation = Vector3::zeros();
    self.should_jump = false;
  }

  pub fn input(&mut self, input: &DefaultInput) {
    if self.id.is_nil() {
      return;
    }

    match input {
      DefaultInput {
        mouse_lock: true,
        delta,
        ..
      } => {
        if delta.x.abs() < 100.0 {
          self.delta_rotation.y = -delta.x;
        }
      }
      _ => {}
    };

    match input {
      DefaultInput {
        mouse_lock: true,
        forward,
        right,
        action,
        ..
      } => {
        self.delta_translation.z += forward;
        self.delta_translation.x += right;

        if self.delta_translation.magnitude_squared() > 0.0 {
          self.delta_translation = self.delta_translation.normalize();
        }

        if *action {
          self.should_jump = true;
        }
      }
      _ => {}
    }
  }

  pub fn update(&mut self, scene: &Scene, delta_in_seconds: f32) {
    if self.id.is_nil() {
      return;
    }

    let mut query = scene.query::<(
      &mut TransformComponent,
      &InputComponent,
      Option<&mut PhysicsComponent>,
      Option<&mut AnimationComponent>,
    )>();

    if let Some((transform, free_range, maybe_physics, maybe_animation)) =
      query.view().get_mut(self.entity)
    {
      let direction = transform.get_euler_direction();

      if let Some(physics) = maybe_physics {
        physics.delta_translation += direction.into_inner().cross(&Vector3::y()).normalize()
          * free_range.speed.as_meters_per_second()
          * self.delta_translation.x;

        physics.delta_translation += direction.into_inner()
          * free_range.speed.as_meters_per_second()
          * self.delta_translation.z;

        physics.delta_rotation += self.delta_rotation * 20.0 * delta_in_seconds;

        if self.should_jump && self.current_jumps < MAX_JUMPS {
          // animation.current = animation.jump;

          self.current_jumps += 1;
          self.delta_velocity.y += JUMP_VELOCITY * 1.0;

          physics.delta_translation += self.delta_velocity;

          self.should_jump = false;
        }
      }

      if self.is_grounded {
        if let Some(animation) = maybe_animation {
          if self.delta_translation.magnitude_squared() > 0.0 {
            animation.current = animation.running;
          } else {
            animation.current = animation.idle;
          }
        }
      }
    }

    self.reset_input();
  }

  pub fn on_collision_start(&mut self, collider_handle: ColliderHandle) {
    // TODO(Pedro): we should get the entity itself and compare it with the tag 'ground'
    self.is_grounded = true;
    self.current_jumps = 0;

    log::info!("start {:?}", collider_handle);
  }

  pub fn on_collision_stop(&mut self, collider_handle: ColliderHandle) {
    // TODO(Pedro): we should get the entity itself and compare it with the tag 'ground'
    self.is_grounded = false;
    log::info!("stop {:?}", collider_handle);
  }
}
