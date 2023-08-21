use crate::shared::weapon::Weapon;

pub struct Cleric {}

impl Weapon for Cleric {
  fn shield(&self) -> f32 {
    4.0
  }

  fn health(&self) -> f32 {
    0.0
  }
}

impl Cleric {
  pub fn new() -> Self {
    Self {}
  }
}
