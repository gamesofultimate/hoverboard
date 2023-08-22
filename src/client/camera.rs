use nalgebra::{Vector3, Point3, Isometry3, Unit};
use engine::{
  application::{
    scene::Scene,
    components::{
      TransformComponent,
      LightComponent,
      CameraComponent,
      IdComponent,
    },
  },
  systems::{
    System, Inventory, Backpack, Initializable,
    rendering::CameraConfig,
  },
  utils::units::Radian,
};

pub struct CameraSystem {
}

impl Initializable for CameraSystem {
  fn initialize(_: &Inventory) -> Self {
    Self {
    }
  }
}

impl System for CameraSystem {
  fn run(&mut self, scene: &mut Scene, backpack: &mut Backpack) {
    for (_, (id, transform, camera)) in &mut scene.query::<(&IdComponent, &TransformComponent, &CameraComponent)>() {
      if !id.is_self { continue }

      let eye_direction = transform.get_euler_direction();
      log::info!("camera system called");

      let offset = (eye_direction.into_inner() * -0.45) + Vector3::new(0.0, 0.3, 0.0);
      let character_position = Point3::from(transform.translation + Vector3::new(0.0, 0.05, 0.0));
      let camera_position = Point3::from(transform.translation + offset);
      let isometry = Isometry3::look_at_rh(&camera_position, &character_position, &Vector3::y());
      let view = isometry.to_homogeneous();

      if let CameraComponent::Perspective { width, height, fovy, zfar, znear, .. } = camera
        && let Some(camera) = backpack.get_mut::<CameraConfig>()
      {
        camera.dimensions.width = *width as u32;
        camera.dimensions.height = *height as u32;
        camera.fovy = *fovy;
        camera.znear = *znear;
        camera.zfar = *zfar;
        //camera.view = view;
        camera.translation = transform.translation + offset;
        camera.front = eye_direction;
        camera.up = Unit::new_normalize(Vector3::y());
      }
    }
  }
}
