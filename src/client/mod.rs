mod camera;
mod world;

use engine::{
  application::{
    bus::BrowserBus,
    components::Prefab,
    input::{DefaultInput, TrustedInput},
  },
  systems::{
    hdr::HdrMultiplayerPipeline, network::NetworkPlugin, rendering::RenderingPlugin,
    trusty::TrustySystem, Scheduler,
  },
  utils::browser::grow_memory,
};

use crate::shared::input::PlayerInput;

use crate::shared::systems::player_movement::PlayerMovementSystem;

// 4k
/*
const DEFAULT_WIDTH:u32 = 3840;
const DEFAULT_HEIGHT:u32 = 2160;
*/
// 1080p
const DEFAULT_WIDTH: u32 = 1920;
const DEFAULT_HEIGHT: u32 = 1080;
/*
*/
/* 720p
const DEFAULT_WIDTH:u32 = 1280;
const DEFAULT_HEIGHT:u32 = 720;
*/

const FRAMES_PER_SECOND: u64 = 60;
const GROW_MEMORY_IN_MB: u32 = 200;

pub async fn main(
  canvas_id: String,
  assets_location: String,
  bus: BrowserBus,
  session_id: String,
  access_token: String,
  udp_url: String,
  tcp_url: String,
) {
  wasm_logger::init(wasm_logger::Config::default());
  grow_memory(GROW_MEMORY_IN_MB);
  let mut runner = Scheduler::new(FRAMES_PER_SECOND, canvas_id);

  log::debug!("assets location: {:?}", &assets_location);
  // let hdr = HdrMultiplayerPipeline::<Prefab, DefaultInput>::new(
  //   assets_location,
  //   session_id,
  //   access_token,
  //   udp_url,
  //   tcp_url,
  // );

  let hdr = HdrMultiplayerPipeline::<Prefab, PlayerInput>::new(
    assets_location,
    session_id,
    access_token,
    udp_url,
    tcp_url,
  );

  runner.attach_plugin(hdr);
  runner.attach_system::<world::WorldSystem>();
  runner.attach_system::<camera::CameraSystem>();
  runner.attach_system::<PlayerMovementSystem>();
  runner.run().await;
}
