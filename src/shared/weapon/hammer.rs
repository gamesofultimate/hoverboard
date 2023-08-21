use crate::shared::weapon::Weapon;

pub struct Hammer {}

impl Weapon for Hammer {
  fn shield(&self) -> f32 {
    0.0
  }

  fn health(&self) -> f32 {
    2.0
  }
}

impl Hammer {
  pub fn new() -> Self {
    Self {}
  }
}
