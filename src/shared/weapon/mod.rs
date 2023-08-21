pub mod cleric;
pub mod great_ax;
pub mod hammer;
pub mod tank;

pub trait Weapon: Sync + Send {
  fn shield(&self) -> f32 {
    0.0
  }

  fn health(&self) -> f32 {
    0.0
  }

  fn primary_ability(&self) {}

  fn secondary_ability(&self) {}

  fn ultimate_ability(&self) {}
}
