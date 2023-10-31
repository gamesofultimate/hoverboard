use engine::{
  application::{
    components::{CameraComponent, LightComponent, SelfComponent},
    scene::{IdComponent, Scene, TransformComponent},
  },
  systems::{rendering::CameraConfig, Backpack, Initializable, Inventory, System},
  utils::units::Radians,
};
use nalgebra::{Isometry3, Point3, Unit, Vector3};

pub struct CameraSystem {}

impl Initializable for CameraSystem {
  fn initialize(_: &Inventory) -> Self {
    Self {}
  }
}

impl System for CameraSystem {
  fn run(&mut self, scene: &mut Scene, backpack: &mut Backpack) {
    for (_, (id, transform, camera, _)) in &mut scene.query::<(
      &IdComponent,
      &TransformComponent,
      &CameraComponent,
      &SelfComponent,
    )>() {
      let eye_direction = transform.get_euler_direction();

      let offset = (eye_direction.into_inner() * -5.) + Vector3::new(0.0, 0.75, 0.0);
      let character_position = Point3::from(transform.translation + Vector3::new(0.0, 0.05, 0.0));
      let camera_position = Point3::from(transform.translation + offset);
      let isometry = Isometry3::look_at_rh(&camera_position, &character_position, &Vector3::y());
      let view = isometry.to_homogeneous();

      if let CameraComponent::Perspective { fovy, zfar, znear, .. } = camera
        && let Some(camera) = backpack.get_mut::<CameraConfig>()
      {
        camera.fovy = *fovy;
        camera.znear = *znear;
        camera.zfar = *zfar;
        camera.translation = transform.translation + offset;
        camera.front = eye_direction;
        camera.up = Unit::new_normalize(Vector3::y());
      }
    }
  }
}
