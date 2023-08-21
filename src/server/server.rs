use async_trait::async_trait;
use networking::server::connection::{Bus, PlayerId};
use std::{collections::HashMap, collections::VecDeque};
use uuid::Uuid;

use engine::{
  application::{
    components::{
      AnimationComponent,
      Asset,
      AudioListenerComponent,

      CameraComponent,
      FullPrefab,
      Gamefile,

      IdComponent,
      InputComponent, // should be free range
      LightComponent,
      ModelComponent,
      ParentComponent,
      ParticleComponent,
      PhysicsComponent,
      PrefabComponent,
      ServerMessage,
      SkyLightComponent,
      TagComponent,
      TerrainComponent,
      TextComponent,
      TransformComponent,
    },
    config::Config,
    downloader::DownloadSender,
    input::TrustedInput,
    layer::Layer,
    physics3d::Physics3d,
    scene::{Entity, Scene},
  },
  get_component,
  renderer::resources::{
    animation::{Animation, AnimationDefinition, AnimationId},
    background::DynamicDefinition,
    fs::Resources,
    model::{Model, ModelDefinition, ModelId},
    terrain::TerrainDefinition,
  },
  utils::units::Radian,
};
use nalgebra::Matrix4;

const MAX_PARENTS: u32 = 4;

// pub enum ArenaEvents {}

#[derive(Debug, Eq, PartialEq, Hash)]
enum AnimationNames {
  // Run,
}

#[derive(Debug, Eq, PartialEq, Hash)]
enum ModelNames {
  Foxy,
  Spectator,
}

#[derive(Debug)]
enum Background {
  // Image { id: Uuid, source: String },
  Dynamic { id: Uuid },
}

pub struct ArenaLayer {
  config: Config,
  resources: Resources,
  scene: Scene,
  physics: Physics3d,
  frame: u128,
  timing: f32,
  // gameplay: Gameplay,
  models: HashMap<ModelId, Model>,
  // model_names: HashMap<ModelNames, Uuid>,
  animations: HashMap<AnimationId, Animation>,
  // animation_names: HashMap<AnimationNames, Uuid>,
  backgrounds: HashMap<Uuid, Background>,
  terrains: HashMap<Uuid, TerrainDefinition>,

  spectator_points: Vec<TransformComponent>,
  spawn_points: Vec<TransformComponent>,
  assigned_spawns: Vec<Option<PlayerId>>,
  prefabs: HashMap<ModelNames, FullPrefab>,

  player_info: HashMap<PlayerId, Entity>,
  entities: HashMap<Uuid, Entity>,
  input_lock: HashMap<PlayerId, u32>,

  // collision_receiver: crossbeam::channel::Receiver<CollisionEvent>,
  // despawn_receiver: crossbeam::channel::Receiver<(Entity, ColliderHandle)>,
  // despawn_sender: crossbeam::channel::Sender<(Entity, ColliderHandle)>,
  events: crossbeam::channel::Receiver<(PlayerId, VecDeque<TrustedInput>)>,
  download_sender: DownloadSender,
  networking: Bus,
}

impl ArenaLayer {
  pub fn new(
    resources: Resources,
    events: crossbeam::channel::Receiver<(PlayerId, VecDeque<TrustedInput>)>,
    download_sender: DownloadSender,
    networking: Bus,
  ) -> Self {
    let scene = Scene::new();

    let (collision_sender, _collision_receiver) = crossbeam::channel::unbounded();
    let (force_sender, _force_receiver) = crossbeam::channel::unbounded();
    // let (despawn_sender, despawn_receiver) = crossbeam::channel::unbounded();

    Self {
      scene,
      config: Config::new(),
      resources: resources.clone(),
      frame: 0,
      timing: 0.0,
      // gameplay: Gameplay::new(),
      physics: Physics3d::new(collision_sender, force_sender),
      events,
      models: HashMap::new(),
      // model_names: HashMap::new(),
      animations: HashMap::new(),
      player_info: HashMap::new(),
      prefabs: HashMap::new(),
      entities: HashMap::new(),
      input_lock: HashMap::new(),
      // animation_names: HashMap::new(),
      backgrounds: HashMap::new(),
      terrains: HashMap::new(),
      spectator_points: vec![],
      spawn_points: vec![],
      assigned_spawns: vec![],
      // collision_receiver,
      // despawn_receiver,
      // despawn_sender,
      download_sender,
      networking,
    }
  }

  fn update_player(&self, player_id: &PlayerId, scene: Vec<FullPrefab>, assets: Vec<Asset>) {
    let character_sync = ServerMessage::SyncWorld {
      scene,
      assets,
      trigger_loading: false,
      frame: self.frame,
    };

    self
      .networking
      .send_reliable_with(*player_id, &character_sync);
  }

  fn sync_world(&self, player_id: &PlayerId) {
    let mut scene = vec![];
    let mut assets = vec![];

    for entity in self.scene.iter() {
      let id = get_component!(self.scene, IdComponent, entity).unwrap();
      let tag = get_component!(self.scene, TagComponent, entity).unwrap();
      let transform = get_component!(self.scene, TransformComponent, entity);
      let animation = get_component!(self.scene, AnimationComponent, entity);
      let physics = get_component!(self.scene, PhysicsComponent, entity);
      let sky = get_component!(self.scene, SkyLightComponent, entity);
      let model = get_component!(self.scene, ModelComponent, entity);
      let prefab = get_component!(self.scene, PrefabComponent, entity);
      let terrain = get_component!(self.scene, TerrainComponent, entity);
      let input = get_component!(self.scene, InputComponent, entity);
      let camera = get_component!(self.scene, CameraComponent, entity);
      let text = get_component!(self.scene, TextComponent, entity);
      let light = get_component!(self.scene, LightComponent, entity);
      let parent = get_component!(self.scene, ParentComponent, entity);
      let particle = get_component!(self.scene, ParticleComponent, entity);
      let audio_listener = get_component!(self.scene, AudioListenerComponent, entity);

      scene.push(FullPrefab {
        id,
        tag,
        transform,
        animation,
        physics,
        sky,
        model,
        prefab,
        terrain,
        input,
        camera,
        text,
        particle,
        light,
        parent,
        audio_listener,
      });
    }

    for (_, asset) in &self.models {
      let mut submeshes = vec![];
      for mesh in asset.into_iter() {
        submeshes.push(mesh.into());
      }
      assets.push(Asset::Model(ModelDefinition {
        id: asset.id,
        source: asset.source.to_string(),
        max_instances: asset.max_instances,
        submeshes,
      }));
    }

    for (_, asset) in &self.terrains {
      assets.push(Asset::Terrain(TerrainDefinition {
        id: asset.id,
        source: asset.source.to_string(),
        tiles: asset.tiles.clone(),
        triangles: asset.triangles,
        lods: asset.lods,
      }));
    }

    for (_, asset) in &self.animations {
      assets.push(Asset::Animation(AnimationDefinition {
        id: asset.id,
        name: asset.name.to_string(),
        source: asset.source.to_string(),
        index: asset.index,
      }));
    }

    for (_, asset) in &self.backgrounds {
      match asset {
        Background::Dynamic { id } => {
          assets.push(Asset::DynamicBackground(DynamicDefinition { id: *id }))
        }
      }
    }

    log::info!("physics: {:?}", self.physics.to_serialize());

    let sync_world = ServerMessage::SyncWorld {
      scene,
      assets,
      trigger_loading: true,
      frame: self.frame,
    };
    log::info!(
      "sending sync event: {:#?}",
      serde_json::to_string(&sync_world).unwrap()
    );
    //log::info!("sending sync event: {:?}", &sync_world);
    self.networking.send_reliable_with(*player_id, &sync_world);
  }

  fn update_config(&self, player_id: &PlayerId) {
    self.networking.send_reliable_with(
      *player_id,
      &ServerMessage::UpdateConfig {
        config: self.config.clone(),
      },
    );
  }

  fn receive_prefab(&mut self, prefab: &FullPrefab) -> (Entity, Uuid) {
    let id = prefab.id.id;
    let entity = match self.entities.get(&prefab.id) {
      Some(entity) => *entity,
      None => {
        let entity = self.scene.create_entity(&prefab.tag.name);
        self.entities.insert(prefab.id.id, entity);
        entity
      }
    };

    self.scene.add_component(entity, prefab.id.clone());
    self.scene.add_component(entity, prefab.tag.clone());
    if let Some(component) = &prefab.transform {
      self.scene.add_component(entity, component.clone())
    }
    if let Some(component) = &prefab.animation {
      self.scene.add_component(entity, component.clone())
    }
    if let Some(component) = &prefab.sky {
      self.scene.add_component(entity, component.clone())
    }
    if let Some(component) = &prefab.model {
      self.scene.add_component(entity, component.clone())
    }
    if let Some(component) = &prefab.terrain {
      self.scene.add_component(entity, component.clone())
    }
    if let Some(component) = &prefab.input {
      self.scene.add_component(entity, component.clone())
    }
    if let Some(component) = &prefab.text {
      self.scene.add_component(entity, component.clone())
    }
    if let Some(component) = &prefab.light {
      self.scene.add_component(entity, component.clone())
    }
    if let Some(component) = &prefab.parent {
      self.scene.add_component(entity, component.clone())
    }
    if let Some(component) = &prefab.physics {
      self.scene.add_component(entity, component.clone())
    }
    if let Some(component) = &prefab.audio_listener {
      self.scene.add_component(entity, component.clone())
    }
    if let Some(component) = &prefab.camera {
      self.scene.add_component(entity, component.clone())
    }
    (entity, id)
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
impl Layer for ArenaLayer {
  async fn on_attach(&mut self) {
    {
      let gamefile = Gamefile::from_file(&self.resources, "arena.lvl").await;
      //let gamefile = Gamefile::from_file(&self.resources, "fps.lvl").await;
      //log::info!("loaded: {:#?}", gamefile);
      self.config = gamefile.config.clone();

      for (id, file_model) in gamefile.bag.models {
        let mut model =
          Model::load_model(&self.download_sender, &self.resources, &file_model.source)
            .await
            .unwrap();
        model.id = id;
        model.max_instances = 100;
        //model.max_instances = file_model.max_instances;
        self.models.insert(id, model);
      }

      for (id, _file_background) in gamefile.bag.dynamic_backgrounds {
        let background = Background::Dynamic { id };
        self.backgrounds.insert(id, background);
      }

      for (_, prefab) in gamefile.bag.entities {
        match prefab.tag.name.as_str() {
          "spectator-spawn-1" => {
            log::info!("creating spectator points {:?}", prefab.tag.name);
            self.spectator_points.push(prefab.transform.unwrap());
          }
          "spawn 1" | "spawn 2" | "spawn 3" | "spawn 4" => {
            log::info!("creating spawn points {:?}", prefab.tag.name);
            self.spawn_points.push(prefab.transform.unwrap());
            self.assigned_spawns.push(None);
          }
          "spectator" => {
            log::info!("creating entity: {:?}", prefab.tag.name);
            self.prefabs.insert(ModelNames::Spectator, prefab.clone());
          }
          "foxy" => {
            log::info!("creating entity: {:?}", prefab.tag.name);
            self.prefabs.insert(ModelNames::Foxy, prefab.clone());
          }
          _ => {
            log::info!("creating entity {:?}", prefab.tag.name);
            self.receive_prefab(&prefab);
          }
        }
      }

      let spectator_prefab = self.prefabs.get(&ModelNames::Spectator).unwrap().clone();
      let mut spectators = vec![];
      for point in &self.spectator_points {
        let mut prefab = spectator_prefab.clone();
        prefab.id.id = Uuid::new_v4();
        prefab.transform = Some(*point);
        spectators.push(prefab);
        //let local_transform = prefab.transform.clone().unwrap();
      }

      for spectator in spectators.drain(..) {
        self.receive_prefab(&spectator);
      }

      for (id, file_animation) in gamefile.bag.animations {
        let index = file_animation.index;
        let name = file_animation.name;
        let source = file_animation.source;
        let mut animation = Animation::load_animation(
          &self.download_sender,
          &self.resources,
          index as u16,
          &name,
          &source,
        )
        .await
        .unwrap();
        animation.id = id;
        self.animations.insert(id, animation);
      }
    }
  }

  fn on_detach(&self) {}

  fn on_player_left(&mut self, player_id: PlayerId) {
    log::info!("[on player left] Player left {player_id:?}");

    if let Some(entity) = self.player_info.remove(&player_id) {
      let _ = self.scene.despawn(entity);
    };
    self.input_lock.remove(&player_id);

    for assigned_spawn in &mut self.assigned_spawns {
      if *assigned_spawn == Some(player_id) {
        *assigned_spawn = None
      }
    }
  }

  fn on_player_joined(&mut self, player_id: PlayerId) {
    self.update_config(&player_id);
    self.sync_world(&player_id);
    {
      let mut prefab: FullPrefab = self.prefabs.get(&ModelNames::Foxy).unwrap().clone();

      let mut spawn_index = 0;
      for (index, assigned_spawn) in self.assigned_spawns.iter_mut().enumerate() {
        if *assigned_spawn == None {
          *assigned_spawn = Some(player_id);
          spawn_index = index;
          break;
        }
      }

      let spawn = self.spawn_points[spawn_index];

      let _local_transform = prefab.transform.clone().unwrap();
      prefab.id.id = *player_id;
      prefab.transform.unwrap().translation = spawn.translation;
      prefab.transform.unwrap().rotation = spawn.rotation;

      //prefab.physics = None;
      //log::info!("transform: {:?}", spawn.clone());

      let (entity, _id) = self.receive_prefab(&prefab);
      self.player_info.insert(player_id, entity);
      self.input_lock.insert(player_id, 0);
      self.entities.insert(*player_id, entity);

      // Update everyone of the new player
      // before we add it to the list of players
      for (room_player_id, _network_entity) in &self.player_info {
        let mut prefab = prefab.clone();
        prefab.id.is_self = *room_player_id == player_id;
        let scene = vec![prefab];
        let assets = vec![];

        self.update_player(room_player_id, scene, assets);
      }
    }
  }

  fn on_update(&mut self, _delta_in_seconds: f32) {
    'input_loop: while let Ok((player_id, inputs)) = &self.events.try_recv() {
      let player_entity = match self.player_info.get(player_id) {
        Some(player) => player,
        None => continue 'input_loop,
      };

      let input_lock = match self.input_lock.get_mut(player_id) {
        Some(value) => value,
        None => continue 'input_loop,
      };

      'lock_loop: for input in inputs {
        if input.frame < *input_lock {
          continue 'lock_loop;
        }

        *input_lock = input.frame;

        let mut query = self
          .scene
          .query::<(&TagComponent, &mut TransformComponent)>();
        let mut view = query.view();
        if let Some((_tag, mut transform)) = view.get_mut(*player_entity) {
          transform.translation = input.translation;
        }
      }

      for (network_id, _network_entity) in &self.player_info {
        // Let's trust each client did the right thing for now
        let message = ServerMessage::Trusted {
          player_id: **player_id,
          inputs: inputs.clone(),
        };
        if player_id != network_id {
          self.networking.send_unreliable_with(*network_id, &message);
        }
      }
    }

    for (_id, (_transform, sky)) in self
      .scene
      .query_mut::<(&TransformComponent, &mut SkyLightComponent)>()
    {
      match sky {
        SkyLightComponent::Dynamic {
          id: _,
          intensity: _,
          turbidity: _,
          azimuth: _,
          inclination,
        } => {
          //*azimuth = Radian::from_degree(self.timing.cos() * 180.0);
          *inclination = Radian::from_degree(self.timing.cos() * 90.0);
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

    let frame_step = 0.0005;
    self.timing += frame_step;
    self.frame += 1;
  }

  fn on_physics(&mut self, _delta_in_seconds: f32) {
    /*
    'physics_loop: for (entity, (transform, physics)) in self.scene.query_mut::<(&mut TransformComponent, &mut PhysicsComponent)>() {
      if transform.translation.y < -700.0 {
        //log::info!("{entity:?}, {:?}: {:?}", physics.collider_handle, transform.translation);
        if let Some(collider) = physics.collider_handle {
          self.despawn_sender.send((entity, collider));
          continue 'physics_loop;
        }
      }

      if let None = physics.body_handle {
        // this physics component has no body yet. Let's create it.
        // We do this at this time, so that we can spawn new entities
        // during run-time.

        // should be a method in `physics3d`
        let rigid_body_def = physics.get_rigid_body(transform.translation.x, transform.translation.y, transform.translation.z);
        let collider_def = physics.get_collider(entity.id() as u128);

        let body_handle = self.physics.bodies.insert(rigid_body_def);
        physics.body_handle = Some(body_handle);

        let collider_handle = self.physics.colliders.insert_with_parent(
          collider_def,
          body_handle,
          &mut self.physics.bodies,
        );

        log::info!("creating physics handlers");

        physics.collider_handle = Some(collider_handle);
      }

      if let Some(handle) = physics.body_handle {
        if let Some(rigid_body) = self.physics.bodies.get_mut(handle) {
          if physics.delta_translation != Vector3::zeros() {
            rigid_body.set_linvel(
              physics.delta_translation,
              true,
            );
          }
          if physics.delta_rotation != Vector3::zeros() {
            rigid_body.set_angvel(
              physics.delta_rotation,
              true,
            );
          }
        }
      }
    }

    while let Ok((entity, collider)) = self.despawn_receiver.try_recv() {
      log::info!("REMOVING! {entity:?}, {:?}", collider);
      self.scene.despawn(entity);
      self.physics.remove(collider);
    }

    self.physics.run(delta_in_seconds);

    for (entity, (transform, physics)) in self.scene.query_mut::<(&mut TransformComponent, &mut PhysicsComponent)>() {
      if let Some(handle) = physics.body_handle {
        if let Some(rigid_body) = self.physics.bodies.get_mut(handle) {
          let translation = rigid_body.position().translation;
          transform.translation.x = translation.x;
          transform.translation.y = translation.y;
          transform.translation.z = translation.z;

          physics.delta_translation.x = 0.0;
          physics.delta_translation.y = 0.0;
          physics.delta_translation.z = 0.0;

          physics.delta_rotation.x = 0.0;
          physics.delta_rotation.y = 0.0;
          physics.delta_rotation.z = 0.0;
        }
      }
    }
    */

    // We should only send things that are relevant to the player
    //#[cfg(not(feature = "client-side-only"))]
    /*
    for (entity, (
      id,
      tag,
      transform,
      camera,
      animation,
      sky,
      input,
      light,
    )) in self.scene.query_mut::<(
      &IdComponent,
      &TagComponent,
      Option<&TransformComponent>,
      Option<&CameraComponent>,
      Option<&AnimationComponent>,
      Option<&SkyLightComponent>,
      Option<&InputComponent>,
      Option<&LightComponent>,
    )>() {
      // TODO: too many clones!!!
      let prefab = ServerMessage::Packet {
        entity,
        frame: self.frame,
        prefab: PartialPrefab {
          id: *id,
          tag: tag.clone(),
          transform: transform.cloned(),
          animation: animation.cloned(),
          sky: sky.cloned(),
          camera: camera.cloned(),
          input: input.cloned(),
          light: light.cloned(),
        }
      };

      for (player_id, network_entity) in &self.player_info {
        // Let's trust each client did the right thing for now
        if entity != *network_entity {
          self.networking.send_unreliable_with(*player_id, &prefab);
        }
      }
    }
    */
  }

  fn on_draw_scene(&mut self) {}

  fn on_im_gui_render(&self) {}

  fn on_event(&self /*event: &Event*/) {}
}
