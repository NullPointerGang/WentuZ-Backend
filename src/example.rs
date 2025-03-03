mod core;

use core::player::{Player, PlayerEvent, EventHandler};
use core::track::Track;
use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;

// Обработчик событий
struct MyEventHandler;

#[async_trait]
impl EventHandler for MyEventHandler {
    async fn handle_event(&self, event: PlayerEvent) {
        match event {
            PlayerEvent::TrackStart(track) => {
                println!(
                    "[HANDLER] Track started: {}",
                    track.title,
                );
            }
            PlayerEvent::TrackEnd(track) => {
                println!("[HANDLER] Track ended: {}", track.title);
            }
            PlayerEvent::VolumeChanged(vol) => {
                println!("[HANDLER] Volume changed to: {:.2}", vol);
            }
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() {
    let mut player = Player::new();
    
    let track1 = Track {
        title: "Test Track 1".to_string(),
        file_path: Some("track1.flac".to_string()),
        file_data: include_bytes!("test_data/test.flac").to_vec(),
    };

    let track2 = Track {
        title: "Test Track 2".to_string(),
        file_path: Some("track2.flac".to_string()),
        file_data: include_bytes!("test_data/test.flac").to_vec(),
    };

    // Регистрируем обработчик событий
    player.add_event_handler(Arc::new(MyEventHandler)).await;

    // Запускаем "прослушку" событий
    let mut rx = player.subscribe_events();
    tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            println!("[EVENT] Received: {:?}", event);
        }
    });

    player.add_to_queue(track1).await;
    player.add_to_queue(track2).await;

    player.auto_play().await;
    
    tokio::time::sleep(Duration::from_secs(3)).await;

    player.pause().await;
    tokio::time::sleep(Duration::from_secs(3)).await;
    player.resume().await;

    player.set_volume(0.5).await;

    player.seek(Duration::from_secs(30)).await;

    tokio::time::sleep(Duration::from_secs(5)).await;

    player.stop().await;

    player.stop_auto_play().await;
}

