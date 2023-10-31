use async_trait::async_trait;
use engine::application::components::ParentComponent;
use engine::application::components::SelfComponent;
use engine::systems::Backpack;
use engine::{
  application::{
    assets::{AssetPack, Store},
    config::Config,
    downloader::DownloadSender,
    gamefile::Gamefile,
    input::TrustedInput,
    scene::{Prefab, PrefabId, Scene, TransformComponent},
  },
  networking::connection::{PlayerId, Protocol},
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
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Eq, PartialEq, Hash)]
enum ModelNames {
  Spectator,
  Player,
  SmokeBomb,
  Hoverboard,
}

impl ModelNames {
  fn from_str(name: &str) -> Self {
    match name {
      "Spectator" => Self::Spectator,
      "Player" => Self::Player,
      "Smoke Bomb" => Self::SmokeBomb,
      "Hoverboard" => Self::Hoverboard,
      _ => panic!("Unknown model name: {}", name),
    }
  }

  fn to_str(&self) -> &str {
    match self {
      Self::Spectator => "Spectator",
      Self::Player => "Player",
      Self::SmokeBomb => "Smoke Bomb",
      Self::Hoverboard => "Hoverboard",
    }
  }
}

pub struct NetworkController {
  spectator_points: Vec<TransformComponent>,
  spawn_points: Vec<TransformComponent>,
  prefabs: HashMap<ModelNames, Prefab>,
  download_sender: DownloadSender,
  client_sender: ClientSender<TrustedInput>,
  config: Option<Config>,
  store: Store,
}

impl Initializable for NetworkController {
  fn initialize(inventory: &Inventory) -> Self {
    let download_sender = inventory.get::<DownloadSender>().clone();
    let client_sender = inventory.get::<ClientSender<TrustedInput>>().clone();
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

    let mut world_entities = scene
      .iter()
      .map(|item| item.entity())
      .collect::<Vec<Entity>>();

    let entities_data: Vec<_> = scene
      .iter()
      .map(|entity| entity.entity())
      .collect::<Vec<Entity>>();

    let mut entities = vec![];

    for entity in entities_data {
      let mut prefab = Prefab::pack(scene, entity).unwrap();
      let is_self = **player_id == **prefab.id;
      if is_self {
        prefab.components.push(Box::new(SelfComponent {}));
      }
      entities.push(prefab);
    }

    let mut prefabs = vec![];
    for (name, prefab) in scene.iter_prefabs() {
      prefabs.push((name.clone(), prefab.clone()));
    }

    log::info!(
      "SYNC WORLD WITH {:?}\n{:#?}\n{:#?}\n{:#?}",
      &player_id,
      &definitions,
      &entities,
      &prefabs
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
        trigger_loading: true,
      },
    );

    self
      .client_sender
      .send_reliable(*player_id, TrustedInput::Prefabs { prefabs });

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

    self.config = Some(gamefile.config.clone());

    for (id, terrain) in gamefile.scene.terrains {
      self.store.insert_asset(id, terrain);
    }
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

    for (id, prefab) in gamefile.scene.prefabs {
      match prefab.tag.name.as_str() {
        "Player" => {
          log::info!("creating player prefab: {:?}", prefab.tag.name);
          self.prefabs.insert(ModelNames::Player, prefab.clone());
        }
        "Smoke Bomb" => {
          log::info!("creating smoke bomb prefab: {:?}", prefab.tag.name);
          self.prefabs.insert(ModelNames::SmokeBomb, prefab.clone());
        }
        "Hoverboard" => {
          log::info!("creating hoverboard prefab: {:?}", prefab.tag.name);
          self.prefabs.insert(ModelNames::Hoverboard, prefab.clone());
        }
        _ => {
          log::info!("receiving entity {:?}", prefab.tag.name);
          let entity = scene.create_raw_entity("tmp");
          scene.create_with_prefab(entity, prefab);
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
    let mut player_prefab: Prefab = self.prefabs.get(&ModelNames::Player).unwrap().clone();
    log::info!("Player joined! New prefab: {:#?}", &player_prefab);

    *player_prefab.id = PrefabId::with_id(*player_id);
    scene.create_with_prefab(entity, player_prefab);

    // spawn new hoverboard and reparent to new player
    let mut hoverboard_prefab: Prefab = self.prefabs.get(&ModelNames::Hoverboard).unwrap().clone();
    let hoverboard_entity = scene.create_raw_entity("Hoverboard");
    *hoverboard_prefab.id = PrefabId::new();
    scene.create_with_prefab(hoverboard_entity, hoverboard_prefab);

    if let parent_component = scene
      .query_one_mut::<&mut ParentComponent>(hoverboard_entity)
      .unwrap()
    {
      parent_component.id = PrefabId::with_id(*player_id);
    }

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
