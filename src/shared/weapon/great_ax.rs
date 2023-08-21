use crate::shared::weapon::Weapon;

pub struct GreatAx {}

impl Weapon for GreatAx {
  fn shield(&self) -> f32 {
    0.0
  }

  fn health(&self) -> f32 {
    2.0
  }
}

impl GreatAx {
  pub fn new() -> Self {
    GreatAx {}
  }
}
