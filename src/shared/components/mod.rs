use nalgebra::Vector3;

pub struct PlayerMovementComponent {
  pub down_vector: Vector3<f32>,
}

impl PlayerMovementComponent {
  pub fn new() -> Self {
    Self {
      down_vector: Vector3::new(0.0, -1.0, 0.0),
    }
  }
}
