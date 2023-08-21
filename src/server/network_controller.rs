use async_trait::async_trait;
use uuid::Uuid;
use networking::connection::{PlayerId, Protocol};
use std::collections::HashMap;
use nalgebra::Vector3;
use engine::systems::Backpack;
use engine::{
  Entity,
  systems::{
    Inventory,
    Initializable,
    network::{ChannelEvents, ClientSender},
  },
  application::{
    config::Config,
    input::TrustedInput,
    downloader::DownloadSender,
    components::{Gamefile, TransformComponent, Prefab},
    scene::{Scene, UnpackEntity},
    assets::{AssetPack, Store},
  },
  renderer::resources::{
    animation::{Animation, AnimationDefinition, AnimationId},
    background::DynamicDefinition,
    fs::Resources,
    model::{Model, ModelDefinition, ModelId},
    terrain::TerrainDefinition,
  },
};

#[derive(Debug, Eq, PartialEq, Hash)]
enum ModelNames {
  Foxy,
  Spectator,
}

pub struct NetworkController {
  spectator_points: Vec<TransformComponent>,
  spawn_points: Vec<TransformComponent>,
  assigned_spawns: Vec<Option<PlayerId>>,
  prefabs: HashMap<ModelNames, Prefab>,
  download_sender: DownloadSender,
  client_sender: ClientSender<TrustedInput<Prefab>>,
  config: Option<Config>,
  store: Store,
}

impl Initializable for NetworkController {
  fn initialize(inventory: &Inventory) -> Self {
    let download_sender = inventory.get::<DownloadSender>().clone();
    let client_sender = inventory.get::<ClientSender<TrustedInput<Prefab>>>().clone();
    let store = Store::new();
    Self {
      client_sender,
      download_sender,
      store,
      spectator_points: vec![],
      spawn_points: vec![],
      assigned_spawns: vec![],
      prefabs: HashMap::new(),
      config: None,
    }
  }
}

impl NetworkController {
  fn sync_world(&self, scene: &mut Scene, player_id: &PlayerId) {
    let mut definitions = vec![];

    for (id, definition) in self.store.iter_assets() {
      let packed = AssetPack::pack(definition);
      definitions.push(packed);
    }

    let mut entities = vec![];
    for entity in scene.iter() {
      let entity = entity.entity();
      let mut prefab = Prefab::pack(scene, &entity);
      prefab.id.is_self = **player_id == prefab.id.id;
      entities.push(prefab);
    }

    log::info!("SYNC WORLD WITH {:?}\n{:#?}\n{:#?}", &player_id, &definitions, &entities);

    if let Some(config) = &self.config {
      self.client_sender.send_reliable(*player_id, TrustedInput::Config { config: config.clone() });
    }
    self.client_sender.send_reliable(*player_id, TrustedInput::Assets { assets: definitions });
    self.client_sender.send_reliable(*player_id, TrustedInput::Entities { entities });
  }
}

#[async_trait]
impl ChannelEvents for NetworkController {
  fn on_session_start(&mut self, scene: &mut Scene, backpack: &mut Backpack) {
    log::info!("Connected to sidecar!!!");

    let gamefile = Gamefile::from_file(&self.download_sender, "arena.lvl");
    //log::info!("loaded: {:?}", gamefile);
    //let gamefile = Gamefile::from_file(&self.resources, "fps.lvl").await;

    self.config = Some(gamefile.config.clone());

    for (id, model) in gamefile.scene.models {
      self.store.insert_asset(id, model);
    }
    for (id, trimesh) in gamefile.scene.trimeshes {
      self.store.insert_asset(id, trimesh);
    }
    for (id, background) in gamefile.scene.dynamic_backgrounds {
      self.store.insert_asset(id, background);
    }
    for (id, animation) in gamefile.scene.animations {
      self.store.insert_asset(id, animation);
    }

    for (id, prefab) in gamefile.scene.entities {
      match prefab.tag.name.as_str() {
        "spectator-spawn-1" | "spectator-spawn-2" | "spectator-spawn-3" | "spectator-spawn-4" => {
          log::info!("creating spectator points {:?}", prefab.tag.name);
          self.spectator_points.push(prefab.transform.unwrap());
        }
        "spawn 1" | "spawn 2" | "spawn 3" | "spawn 4" => {
          log::info!("creating spawn points {:?}", prefab.tag.name);
          self.spawn_points.push(prefab.transform.unwrap());
          self.assigned_spawns.push(None);
        }
        "spectator" => {
          log::info!("creating prefab: {:?}", prefab.tag.name);
          self.prefabs.insert(ModelNames::Spectator, prefab.clone());
        }
        "foxy" => {
          log::info!("creating foxy prefab: {:?}", prefab.tag.name);
          self.prefabs.insert(ModelNames::Foxy, prefab.clone());
        }
        "arena-collider" => {
          log::info!("receiving entity {:?}", prefab.tag.name);
          let entity = scene.create_entity("tmp");
          prefab.unpack(scene, &entity);
        }
        _ => {
          log::info!("receiving entity {:?}", prefab.tag.name);
          let entity = scene.create_entity("tmp");
          prefab.unpack(scene, &entity);
        }
      }
    }

    /*
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

    */
  }

  fn on_player_joined(&mut self, scene: &mut Scene, backpack: &mut Backpack, entity: Entity, player_id: PlayerId, username: String, protocol: Protocol) {
    let mut prefab: Prefab = self.prefabs.get(&ModelNames::Foxy).unwrap().clone();
    log::info!("Player joined! New prefab: {:#?}", &prefab);

    let mut spawn_index = 0;
    for (index, assigned_spawn) in self.assigned_spawns.iter_mut().enumerate() {
      if *assigned_spawn == None {
        *assigned_spawn = Some(player_id);
        spawn_index = index;
        break;
      }
    }

    let spawn = self.spawn_points[spawn_index];

    prefab.id.id = *player_id;
    if let Some(mut transform) = &mut prefab.transform {
      transform.translation = spawn.translation;
      transform.rotation = spawn.rotation;
    }

    prefab.unpack(scene, &entity);
    self.sync_world(scene, &player_id);

    //let entity = scene.create_entity_with_id(prefab.id.id, &prefab.tag.name);

    /*
    // We need to send some information to clients, but that will be done in a
    // separate PR, so I'm leaving this here for now as a reference
    for (room_player_id, _network_entity) in &self.player_info {
      let mut prefab = prefab.clone();
      prefab.id.is_self = *room_player_id == player_id;
      let scene = vec![prefab];
      let assets = vec![];

      self.update_player(room_player_id, scene, assets);
    }
    */
  }

  fn on_player_left(&mut self, scene: &mut Scene, backpack: &mut Backpack, entity: Entity, player_id: PlayerId, protocol: Protocol) {
    log::info!("[on player left] Player left {player_id:?}");

    for assigned_spawn in &mut self.assigned_spawns {
      if *assigned_spawn == Some(player_id) {
        *assigned_spawn = None
      }
    }
    let _ = scene.despawn(entity);
  }
}

