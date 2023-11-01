use nalgebra::Vector3;
use serde::{Deserialize, Serialize};
use tagged::{Registerable, Schema};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Registerable, Schema)]
pub struct PlayerMovementComponent {
  #[serde(skip, default = "default_down_vector")]
  pub down_vector: Vector3<f32>,

  #[schema(default = "2000.0")]
  pub max_velocity: f32,
  #[schema(default = "200.0")]
  pub acceleration: f32,
  #[schema(default = "110.0")]
  pub rotation_speed: f32,
  #[schema(default = "0.02")]
  pub min_height_from_surface: f32,
  #[schema(default = "0.35")]
  pub max_height_from_surface: f32,
  #[schema(default = "6.0")]
  pub height_from_surface_speed: f32,
  #[schema(default = "150.0")]
  pub deceleration: f32,
}

fn default_down_vector() -> Vector3<f32> {
  Vector3::new(0.0, -1.0, 0.0)
}

impl PlayerMovementComponent {}
