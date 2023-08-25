use async_trait::async_trait;
use engine::systems::Backpack;
use engine::{
  application::{
    assets::{AssetPack, Store},
    components::{Gamefile, Prefab, TransformComponent},
    config::Config,
    downloader::DownloadSender,
    input::TrustedInput,
    scene::{Scene, UnpackEntity},
  },
  renderer::resources::{
    animation::{Animation, AnimationDefinition, AnimationId},
    background::DynamicDefinition,
    fs::Resources,
    model::{Model, ModelDefinition, ModelId},
    terrain::TerrainDefinition,
  },
  systems::{
    network::{ChannelEvents, ClientSender},
    Initializable, Inventory,
  },
  Entity,
};
use nalgebra::Vector3;
use networking::connection::{PlayerId, Protocol};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Eq, PartialEq, Hash)]
enum ModelNames {
  Spectator,
  Player,
  SmokeBomb,
}

pub struct NetworkController {
  spectator_points: Vec<TransformComponent>,
  spawn_points: Vec<TransformComponent>,
  prefabs: HashMap<ModelNames, Prefab>,
  download_sender: DownloadSender,
  client_sender: ClientSender<TrustedInput<Prefab>>,
  config: Option<Config>,
  store: Store,
}

impl Initializable for NetworkController {
  fn initialize(inventory: &Inventory) -> Self {
    let download_sender = inventory.get::<DownloadSender>().clone();
    let client_sender = inventory
      .get::<ClientSender<TrustedInput<Prefab>>>()
      .clone();
    let store = Store::new();
    Self {
      client_sender,
      download_sender,
      store,
      spectator_points: vec![],
      spawn_points: vec![],
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

    log::info!(
      "SYNC WORLD WITH {:?}\n{:#?}\n{:#?}",
      &player_id,
      &definitions,
      &entities
    );

    if let Some(config) = &self.config {
      self.client_sender.send_reliable(
        *player_id,
        TrustedInput::Config {
          config: config.clone(),
        },
      );
    }
    self.client_sender.send_reliable(
      *player_id,
      TrustedInput::Assets {
        assets: definitions,
      },
    );
    self
      .client_sender
      .send_reliable(*player_id, TrustedInput::Entities { entities });
  }
}

#[async_trait]
impl ChannelEvents for NetworkController {
  fn on_session_start(&mut self, scene: &mut Scene, backpack: &mut Backpack) {
    log::info!("Connected to sidecar!!!");

    let gamefile = Gamefile::from_file(&self.download_sender, "arena.lvl");
    log::info!("Gamefile: {:#?}", &gamefile);

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
        "Player" => {
          log::info!("creating player prefab: {:?}", prefab.tag.name);
          self.prefabs.insert(ModelNames::Player, prefab.clone());
        }
        "Smoke Bomb" => {
          log::info!("creating smoke bomb prefab: {:?}", prefab.tag.name);
          self.prefabs.insert(ModelNames::SmokeBomb, prefgit aab.clone());
        }
        _ => {
          log::info!("receiving entity {:?}", prefab.tag.name);
          let entity = scene.create_entity("tmp");
          prefab.unpack(scene, &entity);
        }
      }
    }
  }

  fn on_player_joined(
    &mut self,
    scene: &mut Scene,
    backpack: &mut Backpack,
    entity: Entity,
    player_id: PlayerId,
    username: String,
    protocol: Protocol,
  ) {
    let mut prefab: Prefab = self.prefabs.get(&ModelNames::Player).unwrap().clone();
    log::info!("Player joined! New prefab: {:#?}", &prefab);

    prefab.id.id = *player_id;

    prefab.unpack(scene, &entity);

    self.sync_world(scene, &player_id);
  }

  fn on_player_left(
    &mut self,
    scene: &mut Scene,
    backpack: &mut Backpack,
    entity: Entity,
    player_id: PlayerId,
    protocol: Protocol,
  ) {
    log::info!("[on player left] Player left {player_id:?}");
    let _ = scene.despawn(entity);
  }
}
