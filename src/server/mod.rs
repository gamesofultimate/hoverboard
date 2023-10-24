mod network_controller;

//use std::io::Write;

//use networking::server::connection::{Connection, Message as ServerEvents, PlayerId};

use engine::systems::{
  Scheduler,
  network::{NetworkPlugin, ChannelEvents},
  hdr::HdrPipeline,
};

use engine::application::{
  scene::Prefab,
};

use crate::{
  server::{
    network_controller::NetworkController,
  },
  shared::{
    systems::{
      sky::SkySystem,
    },
  },
};

const FRAMES_PER_SECOND: u64 = 60;

pub async fn main() {
  dotenv::dotenv().ok();
  env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

  let rpc_address = {
    let address = dotenv::var("RPC_ADDRESS").unwrap();
    let port = dotenv::var("RPC_PORT").unwrap();

    format!("{}:{}", address, port).parse().unwrap()
  };

  let session_address = {
    let address = dotenv::var("GAME_ADDRESS").unwrap();
    let port = dotenv::var("GAME_PORT").unwrap();

    format!("{}:{}", address, port).parse().unwrap()
  };

  let (mut hdr, download_sender) = HdrPipeline::<NetworkController>::new("resources", rpc_address, session_address);


  let mut runner = Scheduler::new(FRAMES_PER_SECOND);
  runner.attach_plugin(hdr);
  runner.attach_system::<SkySystem>();

  runner.run().await;

  /*

  log::info!("Listening... {rpc_address}, {session_address}");
  log::info!("Size: {:?}", std::mem::size_of::<[Input; 60]>() as u32);

  let ultimate = Connection::new(rpc_address, session_address, session_address).await;

  let bus = ultimate.get_bus();

  let _lifecycle_bus = bus.clone();
  let loop_bus = bus.clone();
  let game_bus = bus.clone();

  let mut application = Application::new();
  let resources = Resources::from_relative_exe_path(Path::new("resources")).unwrap();
  let (mut download_manager, download_sender) = DownloadManager::new();

  let (server_events_sender, server_events_receiver) =
    crossbeam::channel::unbounded::<ServerEvents>();
  let (input_sender, input_receiver) =
    crossbeam::channel::unbounded::<(PlayerId, VecDeque<TrustedInput>)>();

  let _download_loop = tokio::spawn(async move {
    log::info!("Setting up download listener");
    download_manager.run().await;
  });

  let arena_layer =
    server::ArenaLayer::new(resources, input_receiver, download_sender.clone(), game_bus);
  application.push_layer(Box::new(arena_layer)).await;

  let mut listener = ultimate.listen().await;
  let _main_loop = tokio::spawn(async move {
    let target_frametime = time::Duration::from_micros(1_000_000 / FRAMES_PER_SECOND);
    let _empty_time = time::Duration::from_micros(0);
    let mut last_step = time::Instant::now();
    let mut interval = tokio::time::interval(target_frametime);

    loop {
      interval.tick().await;
      let current_step = time::Instant::now();
      let elapsed_time = current_step - last_step;

      while let Ok(game_event) = server_events_receiver.try_recv() {
        log::info!("Game Event: {:?}", game_event);

        match game_event {
          ServerEvents::PlayerJoined {
            player_id,
            username,
            protocol,
          } => {
            log::info!("player joined: {player_id:?}, {username:?}");
            application.player_joined(player_id, username.to_string(), protocol);
          }
          ServerEvents::PlayerLeft {
            player_id,
            protocol,
          } => {
            log::info!("[event] player left: {player_id:?}, {protocol:?}");
            application.player_left(player_id, protocol);
          }
          _ => {}
        }
      }
      let delta_in_seconds = elapsed_time.as_secs_f32();

      // some work here
      application.update(delta_in_seconds);
      application.physics(delta_in_seconds);
      last_step = current_step;

      let _remainder = if target_frametime >= elapsed_time {
        target_frametime - elapsed_time
      } else {
        time::Duration::from_micros(0)
      };

      //if remainder > empty_time { tokio::time::sleep(remainder).await; }
    }
  });

  let io_loop = tokio::spawn(async move {
    loop {
      if let Ok(message) = listener.recv().await {
        match message {
          ServerEvents::Packet { player_id, data } => {
            let _pre_packet = tokio::time::Instant::now();
            let decoded: VecDeque<TrustedInput> = bincode::deserialize(&data[..]).unwrap();

            let _ = input_sender.send((player_id, decoded));
            let _post_packet = tokio::time::Instant::now();
            /*
            log::info!(
              "packet: {:?}",
              post_packet - pre_packet,
            );
            */
          }
          ServerEvents::PlayerJoined {
            player_id,
            username,
            protocol,
          } => {
            log::info!(
              "player joined ({:?}, {:?}): {:?}",
              player_id,
              protocol,
              username
            );
            let _ = server_events_sender.send(ServerEvents::PlayerJoined {
              player_id,
              username,
              protocol,
            });
            let _ = loop_bus.start_game().await;
          }
          ServerEvents::PlayerLeft {
            player_id,
            protocol,
          } => {
            let _ = server_events_sender.send(ServerEvents::PlayerLeft {
              player_id,
              protocol,
            });
          }
        }
      }
    }
  });

  let _ = io_loop.await;
  */
}
