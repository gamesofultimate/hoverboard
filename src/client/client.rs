use async_trait::async_trait;
use engine::{
  application::{
    animation::{AnimationManager, AnimationMode},
    assets::AssetsRunner,
    bus::Message,
    components::{
      AnimationComponent, Asset, CameraComponent,
      FullPrefab, IdComponent, JointConfig, LightComponent,
      ModelComponent, ParentComponent, PartialPrefab, PhysicsComponent,
      ServerMessage, SkyLightComponent, TagComponent,
      TerrainComponent, TransformComponent,
    },
    input::{Input, TrustedInput},
    layer::Layer,
    mouse::MouseManager,
    physics3d::{
      ColliderHandle, CollisionEvent, FixedJointBuilder, Physics3d, RevoluteJointBuilder,
      SphericalJointBuilder,
    },
    renderer3d::{Dimensions, Renderer3d},
    scene::{Entity, Scene},
  },
  renderer::{
    camera::{Camera, Frustum, PerspectiveCamera},
    context::Context,
    resources::{
      animation::Animation,
      background::DynamicBackground,
      fs::Resources,
      model::{Model, ModelId},
      particles::Particle,
      terrain::Terrain,
    },
    texture::Texture,
  },
  utils::{trimesh::Trimesh3d, units::Radian},
};
use futures::channel::mpsc::UnboundedSender;
use nalgebra::{point, Isometry3, Matrix4, Point3, Unit, Vector3};
use std::collections::VecDeque;
use std::collections::{hash_map::Entry, HashMap};
use uuid::Uuid;

use crate::shared::player_controller::PlayerController;

const MAX_PARENTS: u32 = 4;

pub enum ClientEvents {}

pub struct ClientLayer {
  main_sender: UnboundedSender<Message>,
  resources: Resources,
  animations: AnimationManager,
  assets: AssetsRunner,
  scene: Scene,
  entities: HashMap<Uuid, Entity>,
  terrains: HashMap<Uuid, Terrain>,
  perspective_cameras: HashMap<Uuid, PerspectiveCamera>,
  trimeshes: HashMap<ModelId, Trimesh3d>,
  renderer: Renderer3d,
  physics: Physics3d,
  frame: f32,
  viewport: Dimensions,
  canvas_size: Dimensions,
  cursor_lock: bool,
  debouncer: f32,
  mouse: MouseManager,
  previous_mouse: MouseManager,
  received_models: i16,
  received_trimeshes: i16,
  position_cache: HashMap<Uuid, VecDeque<Vector3<f32>>>,
  input_lock: HashMap<Uuid, u32>,
  //predictions: Predictions<PartialPrefab>,
  parent_list: Vec<(Entity, Vector3<f32>, Vector3<f32>)>,
  collision_receiver: crossbeam::channel::Receiver<CollisionEvent>,
  events: crossbeam::channel::Receiver<Input>,
  networking: crossbeam::channel::Receiver<ServerMessage>,
  despawn_receiver: crossbeam::channel::Receiver<(Entity, ColliderHandle)>,
  despawn_sender: crossbeam::channel::Sender<(Entity, ColliderHandle)>,
  server_sender: crossbeam::channel::Sender<TrustedInput>,
  player_controller: PlayerController,
}

impl ClientLayer {
  pub fn new(
    context: &Context,
    resources: &Resources,
    assets: AssetsRunner,
    width: u32,
    height: u32,
    events: crossbeam::channel::Receiver<Input>,
    networking: crossbeam::channel::Receiver<ServerMessage>,
    server_sender: crossbeam::channel::Sender<TrustedInput>,
    main_sender: UnboundedSender<Message>,
  ) -> Self {
    // let weapon = GreatAx::new();

    let scene = Scene::new();

    let viewport = Dimensions { width, height };
    let canvas_size = Dimensions { width, height };

    let (despawn_sender, despawn_receiver) = crossbeam::channel::unbounded();
    let (collision_sender, collision_receiver) = crossbeam::channel::unbounded();
    let (force_sender, _force_receiver) = crossbeam::channel::unbounded();
    let animations = AnimationManager::new();

    let player_controller = PlayerController::new();

    Self {
      scene,
      resources: resources.clone(),
      //predictions: Predictions::new(),
      assets,
      frame: 0.0,
      renderer: Renderer3d::new(context, viewport, canvas_size, 4096),
      main_sender,
      physics: Physics3d::new(collision_sender, force_sender),
      entities: HashMap::new(),
      trimeshes: HashMap::new(),
      terrains: HashMap::new(),
      perspective_cameras: HashMap::new(),
      input_lock: HashMap::new(),
      position_cache: HashMap::new(),
      despawn_receiver,
      mouse: MouseManager::new(),
      previous_mouse: MouseManager::new(),
      despawn_sender,
      server_sender,
      viewport,
      canvas_size,
      cursor_lock: false,
      received_models: 0,
      received_trimeshes: 0,
      debouncer: 0.0,
      events,
      parent_list: vec![],
      animations,
      collision_receiver,
      networking,
      player_controller,
    }
  }

  fn update_mouse_state(&self) {
    if self.mouse.left_click {
      let _ = self
        .main_sender
        .unbounded_send(Message::CaptureMouse(self.mouse.left_click));
    }
  }

  fn get_or_create_entity(&mut self, id: &Uuid, name: &str) -> Entity {
    match self.entities.get(&id) {
      Some(entity) => *entity,
      None => {
        let entity = self.scene.create_entity(name);
        self.entities.insert(*id, entity);
        entity
      }
    }
  }

  fn receive_packet(&mut self, entity: &Entity, prefab: &PartialPrefab) {
    self.scene.add_component(*entity, prefab.id.clone());
    if let Some(component) = &prefab.transform {
      self.scene.add_component(*entity, component.clone())
    }
    if let Some(component) = &prefab.animation {
      self.scene.add_component(*entity, component.clone())
    }
    if let Some(component) = &prefab.sky {
      self.scene.add_component(*entity, component.clone())
    }
    if let Some(component) = &prefab.input {
      self.scene.add_component(*entity, component.clone())
    }
    if let Some(component) = &prefab.camera {
      self.scene.add_component(*entity, component.clone())
    }
  }

  fn receive_prefab(&mut self, entity: &Entity, prefab: &FullPrefab) {
    self.scene.add_component(*entity, prefab.id.clone());
    self.scene.add_component(*entity, prefab.tag.clone());
    if let Some(component) = &prefab.transform {
      self.scene.add_component(*entity, component.clone())
    }
    if let Some(component) = &prefab.animation {
      self.scene.add_component(*entity, component.clone())
    }
    if let Some(component) = &prefab.sky {
      self.scene.add_component(*entity, component.clone())
    }
    if let Some(component) = &prefab.model {
      self.scene.add_component(*entity, component.clone())
    }
    if let Some(component) = &prefab.terrain {
      self.scene.add_component(*entity, component.clone())
    }
    if let Some(component) = &prefab.input {
      self.scene.add_component(*entity, component.clone())
    }
    if let Some(component) = &prefab.camera {
      self.scene.add_component(*entity, component.clone())
    }
    if let Some(component) = &prefab.text {
      self.scene.add_component(*entity, component.clone())
    }
    if let Some(component) = &prefab.light {
      self.scene.add_component(*entity, component.clone())
    }
    if let Some(component) = &prefab.parent {
      self.scene.add_component(*entity, component.clone())
    }
    if let Some(component) = &prefab.physics {
      self.scene.add_component(*entity, component.clone())
    }
    if let Some(component) = &prefab.audio_listener {
      self.scene.add_component(*entity, component.clone())
    }
    if let Some(component) = &prefab.particle {
      self.scene.add_component(*entity, component.clone())
    }
  }

  fn update_parent_transform(
    &self,
    iteration: u32,
    local_transform: Matrix4<f32>,
    parent: Entity,
  ) -> Matrix4<f32> {
    if iteration > MAX_PARENTS {
      return local_transform;
    }

    match (
      self.scene.get::<TransformComponent>(parent),
      self.scene.get::<ParentComponent>(parent),
    ) {
      (Ok(transform_component), Ok(parent_component)) => {
        let player_entity = match self.entities.get(&parent_component.id) {
          Some(player) => player,
          None => {
            panic!("?????????")
          }
        };

        let parent_transform = transform_component.get_transform();
        self.update_parent_transform(
          iteration + 1,
          parent_transform * local_transform,
          *player_entity,
        )
      }
      _ => local_transform,
    }
  }
}

#[async_trait(?Send)]
impl Layer for ClientLayer {
  async fn on_attach(&mut self, _context: &Context) {}

  fn on_detach(&self) {}

  fn on_network(&mut self, context: &Context) {
    while let Ok(assets) = self.assets.listen_for_results() {
      for asset in assets {
        if asset.is::<DynamicBackground>() {
          let background = asset.downcast::<DynamicBackground>().unwrap();
          let id = background.id;
          log::info!(
            "Received dynamic background, including it into rendering: {:?}",
            id
          );
          // 64 is the width of the texture. We should let the user configure that
          self.renderer.include_dynamic_background(&context, id, 64);
        } else if asset.is::<Trimesh3d>() {
          let trimesh = asset.downcast::<Trimesh3d>().unwrap();
          let trimesh_id = trimesh.id;
          self.received_trimeshes = (self.received_trimeshes - 1).max(0);
          self.trimeshes.insert(trimesh_id, *trimesh);
          log::info!("Received trimesh: {trimesh_id:?}");
        } else if asset.is::<Model>() {
          let model = asset.downcast::<Model>().unwrap();
          let model_id = model.id;
          self.renderer.include_model(*model);
          self.renderer.load_model(&context, &model_id, 100);
          self.received_models = (self.received_models - 1).max(0);
          //log::info!("Received model ({:}): {:?}", max_instances, model_id);
          log::info!(
            "Received model: {model_id:?}, received: {:}",
            self.received_models
          );
        } else if asset.is::<Terrain>() {
          let terrain = asset.downcast::<Terrain>().unwrap();
          let terrain_id = terrain.id;
          self.renderer.include_terrain(context, *terrain);
          self.renderer.load_terrain(terrain_id);
          log::info!("Received terrain: {terrain_id:?}");
        } else if asset.is::<Animation>() {
          let animation = asset.downcast::<Animation>().unwrap();
          log::info!("Received animation: {:?}", animation.id);
          self.renderer.include_animation(*animation);
        } else if asset.is::<Texture>() {
          let texture = asset.downcast::<Texture>().unwrap();
          log::debug!("Received texture: {:?}", texture.id);
          self.renderer.load_texture(*texture);
        } else if asset.is::<Particle>() {
          let particle = asset.downcast::<Particle>().unwrap();
          log::debug!("Received particle: {:?}", particle.id);
          self.renderer.load_particle_asset(*particle);
        }
      }
    }

    while let Ok(message) = self.networking.try_recv() {
      match message {
        ServerMessage::Trusted { player_id, inputs } => {
          let _player_entity = self.get_or_create_entity(&player_id, &"tmp");

          let input_lock = self.input_lock.entry(player_id).or_insert(0);

          'lock_loop: for input in inputs {
            if input.frame < *input_lock {
              continue 'lock_loop;
            }

            *input_lock = input.frame;
            let cache = self
              .position_cache
              .entry(player_id)
              .or_insert(VecDeque::new());
            cache.push_front(input.translation);
          }
        }
        ServerMessage::UpdateConfig { config } => {
          // log::info!("Received new config {:?}", &config);
          self.renderer.set_config(&config);
        }
        ServerMessage::SyncWorld {
          scene,
          assets,
          frame: _,
          trigger_loading,
        } => {
          //log::info!("Sync world: {scene:?}, {assets:?}");
          for asset in assets.clone().into_iter() {
            match asset {
              Asset::Model(model) => {
                log::info!("DOWNLOADING MODEL");
                if trigger_loading {
                  self.received_models += 1;
                  self.received_trimeshes += 1;
                }
                self.assets.senders.request_download(Box::new(model));
              }
              Asset::DynamicBackground(background) => {
                log::info!("DOWNLOADING BACKGROUND");
                self.assets.senders.request_download(Box::new(background));
              }
              Asset::Animation(animation) => {
                log::info!("DOWNLOADING ANIMATION");
                self.assets.senders.request_download(Box::new(animation));
              }
              Asset::Terrain(terrain) => {
                log::info!("DOWNLOADING TERRAIN");
                self.assets.senders.request_download(Box::new(terrain));
              }
              Asset::Texture(texture) => {
                log::info!("DOWNLOADING TEXTURE");
                self.assets.senders.request_download(Box::new(texture));
              }
              Asset::Particle(particle) => {
                log::info!("DOWNLOADING PARTICLE");
                self.assets.senders.request_download(Box::new(particle));
              }
            }
          }

          for prefab in scene.clone().into_iter() {
            let entity = self.get_or_create_entity(&prefab.id.id, &prefab.tag.name);
            self.receive_prefab(&entity, &prefab);
          }
        }
        ServerMessage::Packet { prefab, .. } => {
          let entity = self.get_or_create_entity(&prefab.id.id, &prefab.tag.name);
          self.receive_packet(&entity, &prefab);
        }
        _ => {}
      }
    }
  }

  fn on_update(&mut self, context: &Context, delta_in_seconds: f32) {
    self.renderer.new_frame(context);

    for (player_id, cache) in &mut self.position_cache {
      // TODO: Need to be reabstracted into `get_or_create_entity`
      let player_entity = match self.entities.get(&player_id) {
        Some(entity) => *entity,
        None => {
          let entity = self.scene.create_entity(&"tmp");
          self.entities.insert(*player_id, entity);
          entity
        }
      };

      if let Some(translation) = cache.pop_back() {
        let mut query = self
          .scene
          .query::<(&TagComponent, &mut TransformComponent)>();
        let mut view = query.view();
        if let Some((_tag, mut transform)) = view.get_mut(player_entity) {
          transform.translation = translation;
        }
      }
    }

    for (_entity, camera) in &mut self.scene.query::<&CameraComponent>() {
      match *camera {
        CameraComponent::Perspective {
          id,
          aspect,
          fovy,
          zfar,
          znear,
          ..
        } => {
          match self.perspective_cameras.entry(id) {
            Entry::Occupied(entry) => {
              let entry = entry.into_mut();
              entry.update(aspect, *fovy, znear, zfar);
            }
            Entry::Vacant(spot) => {
              let camera = PerspectiveCamera::with_raw(aspect, *fovy, znear, zfar);
              spot.insert(camera);
            }
          };
        }
        _ => panic!("Camera not implemented yet"),
      }
    }

    if self.player_controller.is_initialized() == false {
      for (entity, id) in &mut self.scene.query::<&IdComponent>() {
        if !id.is_self {
          continue;
        }

        self
          .player_controller
          .initialize(&self.scene, entity, id.id);
      }
    }

    // Input handling
    {
      let inputs = &self.events.try_iter().collect::<Vec<_>>();
      assert!(inputs.len() == 1);

      let input = inputs.first().unwrap();

      self.mouse.left_click = input.left_click;
      self.update_mouse_state();

      match &input {
        Input {
          canvas: (width, height),
          ..
        } => {
          self.canvas_size.width = *width;
          self.canvas_size.height = *height;
        }
      }

      self.player_controller.input(input);
      self.player_controller.update(&self.scene, delta_in_seconds);

      self.previous_mouse = self.mouse;
    }
    // Input handling

    // // let mut directional_intensity = 0.0;
    let mut directional_radiance = Vector3::zeros();

    // let mut sky_intensity = 0.0;
    // let mut sky_turbidity = 0.0;
    let mut sky_azimuth = Radian::from_degree(0.0);
    let mut sky_inclination = Radian::from_degree(0.0);

    for (_id, (_transform, light)) in self
      .scene
      .query_mut::<(&mut TransformComponent, &LightComponent)>()
    {
      match light {
        LightComponent::Directional {
          radiance,
          azimuth,
          inclination,
          intensity: _,
          ..
        } => {
          sky_azimuth = *azimuth;
          // sky_inclination = Radian::from_radian(-PI * self.frame.sin());
          sky_inclination = *inclination;

          directional_radiance = radiance.clone();
          // directional_intensity = intensity.clone();
        }
        _ => {}
      };
    }

    for (_id, (_transform, sky)) in self
      .scene
      .query_mut::<(&TransformComponent, &mut SkyLightComponent)>()
    {
      match sky {
        SkyLightComponent::Dynamic {
          id,
          intensity,
          turbidity,
          azimuth,
          inclination,
        } => {
          // sky_intensity = intensity.clone();
          // sky_turbidity = turbidity.clone();
          *azimuth = sky_azimuth.clone();
          *inclination = sky_inclination.clone();

          self.renderer.setup_dynamic_background(
            *id,
            *intensity,
            *turbidity,
            *azimuth,
            *inclination,
          );
        }
        _ => {}
      }
    }

    for (_id, (transform_component, parent_component)) in &mut self
      .scene
      .query::<(&TransformComponent, &mut ParentComponent)>()
    {
      let local_transform = transform_component.get_transform();

      let player_entity = match self.entities.get(&parent_component.id) {
        Some(player) => player,
        None => {
          panic!("wuut?")
        }
      };

      parent_component.transform = self.update_parent_transform(0, local_transform, *player_entity);
    }

    for (_, sky) in self.scene.query_mut::<&SkyLightComponent>() {
      match sky {
        SkyLightComponent::Image { id, .. } => self.renderer.draw_background(&id),
        SkyLightComponent::Dynamic { id, .. } => self.renderer.draw_background(&id),
      }
    }

    for (_entity, (id, animation)) in &mut self.scene.query::<(&IdComponent, &AnimationComponent)>()
    {
      self
        .animations
        .insert(id.id, animation.current, AnimationMode::Playing);
    }

    let mut frustum = Frustum::empty();

    for (_entity, (id, transform, camera)) in
      &mut self
        .scene
        .query::<(&IdComponent, &mut TransformComponent, &mut CameraComponent)>()
    {
      //log::info!("id: {:?}", &id);
      if !id.is_self {
        continue;
      }

      let direction = transform.get_euler_direction();

      //log::info!("{:}: y: {:}, direction: {:?}", &tag.name, transform.rotation.y, &direction);
      //let view = input.get_transform(&transform.translation);
      let view = {
        //log::info!("direction: {:?} rotation: {:?}", &direction, transform.rotation);
        //let offset = (direction.into_inner() * -4.5) + Vector3::new(0.0, 1.5, 0.0);
        let offset = (direction.into_inner() * -0.45) + Vector3::new(0.0, 0.15, 0.0);
        let car_position = Point3::from(transform.translation + Vector3::new(0.0, 0.05, 0.0));
        let camera_position = Point3::from(transform.translation + offset);
        let iso = Isometry3::look_at_rh(&camera_position, &car_position, &Vector3::y());
        iso.to_homogeneous()
      };

      let sky_direction = Unit::new_normalize(Vector3::new(
        sky_inclination.sin() * sky_azimuth.cos(),
        sky_inclination.cos(),
        sky_inclination.sin() * sky_azimuth.sin(),
      ));

      if let CameraComponent::Perspective { id, .. } = camera {
        let camera = self.perspective_cameras.get_mut(id).unwrap();
        self.renderer.update_canvas(
          self.canvas_size.width as u32,
          self.canvas_size.height as u32,
        );
        camera.set_viewport(
          self.canvas_size.width as f32,
          self.canvas_size.height as f32,
        );
        self
          .renderer
          .set_environment(sky_direction, directional_radiance, camera, &view);

        self
          .renderer
          .load_camera(transform.translation, camera.get_projection(), view);

        let position = {
          Vector3::new(
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
          )
        };

        frustum = camera.get_frustum(position, direction, Unit::new_normalize(Vector3::y()));
      }
    }

    for (_id, (transform, light)) in self
      .scene
      .query_mut::<(&TransformComponent, &LightComponent)>()
    {
      match light {
        LightComponent::Point {
          radiance,
          intensity,
          radius,
          falloff,
          ..
        } => {
          self.renderer.draw_point_light(
            context,
            &frustum,
            *radius,
            *intensity,
            *falloff,
            *radiance,
            transform.translation,
          );
        }
        _ => {}
      };
    }

    for (_, (id, transform, mesh, maybe_animation, maybe_parent)) in self.scene.query_mut::<(
      &IdComponent,
      &mut TransformComponent,
      &ModelComponent,
      Option<&mut AnimationComponent>,
      Option<&mut ParentComponent>,
    )>() {
      match (maybe_parent, maybe_animation) {
        (Some(parent), Some(animation)) => {
          if let Some(frame) = self.animations.get_frame(id.id, animation.current) {
            self.renderer.draw_model_animated(
              &frustum,
              &mesh.id,
              &mesh.submesh_id,
              None,
              parent.transform,
              animation.current,
              frame,
            );
          }
        }
        (Some(parent), None) => {
          self.renderer.draw_model_static(
            &frustum,
            &mesh.id,
            &mesh.submesh_id,
            None,
            parent.transform,
          );
        }
        (None, Some(animation)) => {
          if let Some(frame) = self.animations.get_frame(id.id, animation.current) {
            self.renderer.draw_model_animated(
              &frustum,
              &mesh.id,
              &mesh.submesh_id,
              None,
              transform.get_transform(),
              animation.current,
              frame,
            );
          }
        }
        (None, None) => {
          self.renderer.draw_model_static(
            &frustum,
            &mesh.id,
            &mesh.submesh_id,
            None,
            transform.get_transform(),
          );
        }
      };
    }

    for (entity, (terrain, transform)) in self
      .scene
      .query_mut::<(&TerrainComponent, &TransformComponent)>()
    {
      //log::info!("drawing terrain");
      self.renderer.draw_terrain(
        &frustum,
        terrain.id,
        terrain.height,
        terrain.tile_half_length,
        Some(entity.id()),
        transform.get_transform(),
      );
    }

    self.animations.tick(delta_in_seconds);

    if self.received_models > 0 || self.received_trimeshes > 0 {
      return;
    }

    let frame_step = 0.00005;
    self.frame += frame_step;
    self.debouncer = (self.debouncer + frame_step) % 60.0;

    self.renderer.debug_physics(&mut self.physics);
  }

  fn on_physics(&mut self, delta_in_seconds: f32) {
    if self.received_models > 0 || self.received_trimeshes > 0 {
      return;
    }

    //#[cfg(feature = "client-side-only")]
    {
      'physics_loop: for (entity, (tag, transform, physics)) in self.scene.query_mut::<(
        &TagComponent,
        &mut TransformComponent,
        &mut PhysicsComponent,
      )>() {
        if transform.translation.y < -700.0 {
          if let Some(collider) = self.physics.collider_handles.get(&physics.joint.body.id) {
            log::info!("despawning: {entity:?}: {:?}", transform.translation);
            let _ = self.despawn_sender.send((entity, *collider));
            continue 'physics_loop;
          }
        }

        if let None = self.physics.body_handles.get(&physics.joint.body.id) {
          // this physics component has no body yet. Let's create it.
          // We do this at this time, so that we can spawn new entities
          // during run-time.

          log::info!("creating handler: {:?}", &tag.name);

          for (parent, joint) in physics.iter_with_parent() {
            // should be a method in `physics3d`
            let rigid_body_def = joint.get_rigid_body(match parent {
              Some(_parent) => None,
              None => Some(&transform),
            });
            let collider_def =
              match joint.get_collider(entity.id() as u128, &self.trimeshes, &transform) {
                None => continue 'physics_loop,
                Some(data) => data,
              };

            let body_handle = self.physics.bodies.insert(rigid_body_def);
            self.physics.body_handles.insert(joint.body.id, body_handle);

            let collider_handle = self.physics.colliders.insert_with_parent(
              collider_def,
              body_handle,
              &mut self.physics.bodies,
            );
            self
              .physics
              .collider_handles
              .insert(joint.body.id, collider_handle);

            match (parent, joint.config) {
              (Some(parent), Some(JointConfig::Fixed { local_anchor })) => {
                let parent_body = self.physics.body_handles.get(&parent.body.id).unwrap();
                let sphere = FixedJointBuilder::new()
                  .local_anchor1(point![joint.offset.x, joint.offset.y, joint.offset.z])
                  .local_anchor2(point![local_anchor.x, local_anchor.y, local_anchor.z])
                  .contacts_enabled(false)
                  .build();
                let _handle =
                  self
                    .physics
                    .impulse_joints
                    .insert(*parent_body, body_handle, sphere, true);
                log::info!("FIXED: {:} -- offset: {:?}", &joint.name, &joint.offset);
              }
              (Some(parent), Some(JointConfig::Spherical { local_anchor })) => {
                let parent_body = self.physics.body_handles.get(&parent.body.id).unwrap();
                let sphere = SphericalJointBuilder::new()
                  .local_anchor1(point![joint.offset.x, joint.offset.y, joint.offset.z])
                  .local_anchor2(point![local_anchor.x, local_anchor.y, local_anchor.z])
                  .contacts_enabled(false)
                  .build();
                let _handle =
                  self
                    .physics
                    .impulse_joints
                    .insert(*parent_body, body_handle, sphere, true);
                log::info!("SPHERICAL: {:} -- offset: {:?}", &joint.name, &joint.offset);
              }
              (Some(parent), Some(JointConfig::Revolute { axis, local_anchor })) => {
                let parent_body = self.physics.body_handles.get(&parent.body.id).unwrap();
                let sphere = RevoluteJointBuilder::new(Unit::new_normalize(axis))
                  .local_anchor1(point![joint.offset.x, joint.offset.y, joint.offset.z])
                  .local_anchor2(point![local_anchor.x, local_anchor.y, local_anchor.z])
                  .contacts_enabled(false)
                  .build();
                let _handle =
                  self
                    .physics
                    .impulse_joints
                    .insert(*parent_body, body_handle, sphere, true);
                log::info!("SPHERICAL: {:} -- offset: {:?}", &joint.name, &joint.offset);
              }
              _ => {}
            }
          }
        }

        if let Some(handle) = self.physics.body_handles.get(&physics.joint.body.id) {
          //log::info!("{:?}: physics: {:?}, {:?}", tag.name, transform, physics);
          if let Some(rigid_body) = self.physics.bodies.get_mut(*handle) {
            rigid_body.set_linvel(physics.delta_translation, true);
            rigid_body.set_angvel(physics.delta_rotation, true);
          }
        }
      }

      while let Ok((entity, collider)) = self.despawn_receiver.try_recv() {
        log::info!("REMOVING! {entity:?}, {:?}", collider);
        let _ = self.scene.despawn(entity);
        self.physics.remove(collider);
      }

      for (_entity, (id, physics)) in self
        .scene
        .query_mut::<(&IdComponent, &mut PhysicsComponent)>()
      {
        if id.is_self == false {
          continue;
        }

        let collision_events = &self.collision_receiver.try_iter().collect::<Vec<_>>();

        for event in collision_events {
          if let Some(handle) = self.physics.collider_handles.get(&physics.joint.body.id) {
            if event.collider1() == *handle {
              if event.started() {
                self.player_controller.on_collision_start(event.collider2());
              } else if event.stopped() {
                self.player_controller.on_collision_stop(event.collider2());
              }
            } else if event.collider2() == *handle {
              if event.started() {
                self.player_controller.on_collision_start(event.collider1());
              } else if event.stopped() {
                self.player_controller.on_collision_stop(event.collider1());
              }
            }
          }
        }
      }

      self
        .physics
        .run(delta_in_seconds, &Vector3::new(0.0, -9.81 * 10.0, 0.0));

      for (_entity, (_id, transform, physics)) in
        self
          .scene
          .query_mut::<(&IdComponent, &mut TransformComponent, &mut PhysicsComponent)>()
      {
        if let Some(handle) = self.physics.collider_handles.get(&physics.joint.body.id) {
          if let Some(collider) = self.physics.colliders.get_mut(*handle) {
            let translation = collider.position().translation;
            let rotation = collider.position().rotation.euler_angles();

            transform.translation.x = translation.x;
            transform.translation.y = translation.y;
            transform.translation.z = translation.z;
            transform.translation -= physics.joint.offset;

            transform.rotation.x = rotation.0;
            transform.rotation.y = rotation.1;
            transform.rotation.z = rotation.2;

            physics.delta_translation.x = 0.0;
            physics.delta_translation.y = 0.0;
            physics.delta_translation.z = 0.0;

            physics.delta_rotation.x = 0.0;
            physics.delta_rotation.y = 0.0;
            physics.delta_rotation.z = 0.0;
          }
        }
      }
    }
  }

  fn on_post_physics(&mut self) {
    for (_entity, (id, transform)) in self
      .scene
      .query_mut::<(&IdComponent, &TransformComponent)>()
    {
      if !id.is_self {
        continue;
      }
      // TODO: too many clones!!!
      let trusted_input = TrustedInput {
        frame: 0,
        translation: transform.translation,
      };

      let _ = self.server_sender.send(trusted_input);
    }
  }

  fn on_draw_scene(&mut self, context: &Context) {
    self.renderer.draw_scene(context, None, None);
    self.renderer.clean_up(context);
  }
}
