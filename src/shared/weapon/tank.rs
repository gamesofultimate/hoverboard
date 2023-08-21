use crate::shared::weapon::Weapon;

pub struct Tank {}

impl Weapon for Tank {
  fn shield(&self) -> f32 {
    4.0
  }

  fn health(&self) -> f32 {
    2.0
  }
}

impl Tank {
  pub fn new() -> Self {
    Self {}
  }
}
