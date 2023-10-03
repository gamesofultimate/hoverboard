use serde::{Serialize, Deserialize};
use engine::{
  application::{
    scene::{
      Scene,
      TransformComponent,
      component_registry::Access,
    },
    components::{
      SkyLightComponent,
    },
  },
  systems::{System, Inventory, Backpack, Initializable},
  utils::units::Radian,
};

use tagged::{Registerable, Schema};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Registerable, Schema)]
pub struct SkyComponent {
  pub elevation: f32,
}

pub struct SkySystem {
  timing: f32,
}

impl Initializable for SkySystem {
  fn initialize(_: &Inventory) -> Self {
    Self {
      timing: 0.0,
    }
  }
}

impl System for SkySystem {
  fn provide(&mut self, _: &Inventory) {
    SkyComponent::register();
  }

  fn run(&mut self, scene: &mut Scene, backpack: &mut Backpack) {
    /*
    for (_, sky) in scene.query_mut::<&mut SkyLightComponent>() {
      match sky {
        SkyLightComponent::Dynamic {
          id: _,
          intensity: _,
          turbidity: _,
          azimuth: _,
          inclination,
        } => {
          // *azimuth = Radian::from_degree(self.timing.cos() * 180.0);
          *inclination = Radian::from_degree(self.timing.cos() * 90.0);
        }
        _ => {}
      }
    }
    */

    self.timing += 0.0005;
  }
}

