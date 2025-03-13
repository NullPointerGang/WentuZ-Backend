/**
 * Copyright 2025 FlacSy
 *
 * Licensed under the FlacSy Open Use License (FOUL) 1.0.
 * You may not use this file except in compliance with the License.
 * You may obtain a copy of the License at:
 *
 *     https://github.com/FlacSy/FOUL-LICENSE/blob/main/LICENSE
 *
 * This software is provided "as is", without any warranties, express or implied.
 * The author is not responsible for any damages or losses arising from the use of this software.
 * The License governs the permissions and restrictions related to the use, modification,
 * and distribution of this software.
 *
 * Commercial use is only permitted with prior written consent from the author.
*/

use crate::core::queue::Queue;
use crate::core::track::Track;

use std::time::Duration;
use rodio::{Decoder, OutputStream, Sink, Device};
use rodio::cpal::{self, traits::{HostTrait, DeviceTrait}};
use std::io::Cursor;
use std::sync::Arc;

use tokio::sync::{Mutex, mpsc, broadcast};
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub enum PlayerErrors {
    SinkNotFound,
    FailedDecode,
    QueueEmpty,
    TrackNotFound,
    SeekError,
    DeviceNotFound,
    DeviceError(String),
    UnknownError(String),
}

#[derive(Debug, Clone)]
pub enum PlayerEvent {
    TrackStart(Track),
    TrackEnd(Track),
    Paused,
    Resumed,
    VolumeChanged(f32),
    Seek(Duration),
    Stopped,
    AutoPlayStarted,
    AutoPlayStopped,
    QueueUpdated,
    DeviceChanged(String),
    Error(PlayerErrors),
}

#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle_event(&self, event: PlayerEvent);
}

#[derive(Clone)]
pub struct Player {
    pub queue: Arc<Mutex<Queue>>,
    pub current_sink: Arc<Mutex<Option<Arc<Sink>>>>,
    current_device: Arc<Mutex<Option<Device>>>,
    auto_play_enabled: Arc<Mutex<bool>>,
    auto_play_tx: Option<mpsc::Sender<()>>,
    volume: f32,
    event_tx: broadcast::Sender<PlayerEvent>,
    event_handlers: Arc<Mutex<Vec<Arc<dyn EventHandler>>>>,
}

impl Player {
    pub fn new() -> Self {
        println!("[DEBUG] Initializing player");
        let (event_tx, _) = broadcast::channel(32);
        Player {
            queue: Arc::new(Mutex::new(Queue::new())),
            current_sink: Arc::new(Mutex::new(None)),
            current_device: Arc::new(Mutex::new(None)),
            auto_play_enabled: Arc::new(Mutex::new(false)),
            auto_play_tx: None,
            volume: 1.0,
            event_tx,
            event_handlers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn add_event_handler(&self, handler: Arc<dyn EventHandler>) {
        let mut handlers = self.event_handlers.lock().await;
        handlers.push(handler);
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<PlayerEvent> {
        self.event_tx.subscribe()
    }

    async fn send_event(&self, event: PlayerEvent) {
        let _ = self.event_tx.send(event.clone());
        
        let handlers = self.event_handlers.lock().await;
        for handler in handlers.iter() {
            let event = event.clone();
            let handler = Arc::clone(handler);
            tokio::spawn(async move {
                handler.handle_event(event).await;
            });
        }
    }

    pub async fn seek(&self, position: Duration) {
        println!("[DEBUG] Seeking to position: {:?}", position);
        let sink_guard = self.current_sink.lock().await;
        if let Some(sink) = sink_guard.as_ref() {
            if let Err(e) = sink.try_seek(position) {
                println!("[DEBUG] Failed to seek to position: {:?}, error: {:?}", position, e);
                self.send_event(PlayerEvent::Error(PlayerErrors::SeekError)).await;
            }
        } else {
            println!("[DEBUG] Нет активного воспроизведения для перемотки");
            self.send_event(PlayerEvent::Error(PlayerErrors::SinkNotFound)).await;
        }
    }

    pub async fn stop(&self) {
        println!("[DEBUG] Stopping current playback");
        let mut sink_guard = self.current_sink.lock().await;
        if let Some(sink) = sink_guard.take() {
            sink.stop();
            println!("[DEBUG] Playback stopped");
            self.send_event(PlayerEvent::Stopped).await;
        } else {
            println!("[DEBUG] No active playback to stop");
            self.send_event(PlayerEvent::Error(PlayerErrors::SinkNotFound)).await;
        }
    }

    pub async fn list_devices(&self) -> Result<Vec<String>, PlayerErrors> {
        let host = cpal::default_host();
        match host.output_devices() {
            Ok(devices) => {
                let device_names: Vec<String> = devices
                    .map(|device| device.name().unwrap_or_else(|_| "Unknown Device".to_string()))
                    .collect();
                Ok(device_names)
            }
            Err(e) => Err(PlayerErrors::DeviceError(e.to_string())),
        }
    }

    pub async fn set_device(&self, device_name: &str) -> Result<(), PlayerErrors> {
        println!("[DEBUG] Setting audio device: {}", device_name);
        
        let host = cpal::default_host();
        let mut devices = host.output_devices().map_err(|e| PlayerErrors::DeviceError(e.to_string()))?;
        let device = devices
            .find(|d| d.name().map_or(false, |name| name == device_name))
            .ok_or(PlayerErrors::DeviceNotFound)?;

        let device: Device = device.into();
        let mut device_guard = self.current_device.lock().await;
        *device_guard = Some(device);

        self.send_event(PlayerEvent::DeviceChanged(device_name.to_string())).await;
        Ok(())
    }

    pub async fn get_current_device(&self) -> Option<String> {
        let device_guard = self.current_device.lock().await;
        device_guard.as_ref().and_then(|d| d.name().ok())
    }

    pub async fn play(&self, track: Track) {
        self.stop().await;

        println!(
            "[DEBUG] Starting playback in an async task for track: title={} file_path={:?}",
            track.title, track.file_path
        );

        let track_data = track.file_data.clone();
        let track_title = track.title.clone();
        let current_sink = Arc::clone(&self.current_sink);
        let current_device = Arc::clone(&self.current_device);
        let volume = self.volume;

        tokio::spawn(async move {
            let res = tokio::task::spawn_blocking(move || {
                let cursor = Cursor::new(track_data);
                match Decoder::new_looped(cursor) {
                    Ok(source) => {
                        let device = {
                            let device_guard = current_device.blocking_lock();
                            device_guard.as_ref().cloned()
                        };

                        let (_stream, stream_handle) = match device {
                            Some(device) => OutputStream::try_from_device(&device).unwrap(),
                            None => OutputStream::try_default().unwrap(),
                        };

                        let sink = Arc::new(Sink::try_new(&stream_handle).unwrap());
                        sink.set_volume(volume);

                        {
                            let mut sink_guard = current_sink.blocking_lock();
                            *sink_guard = Some(Arc::clone(&sink));
                        }

                        sink.append(source);

                        println!("[DEBUG] Playback started for track: title={}", track_title);
                        tokio::spawn(async move {
                            let player = Player::new();
                            player.send_event(PlayerEvent::TrackStart(track.clone())).await;
                        });

                        sink.sleep_until_end();

                        println!("[DEBUG] Playback completed for track: title={}", track_title);

                        {
                            let mut sink_guard = current_sink.blocking_lock();
                            *sink_guard = None;
                        }
                    }
                    Err(e) => {
                        println!("[DEBUG] Failed to decode track: title={}, error: {:?}", track_title, e);
                    }
                }
            })
            .await;

            if let Err(e) = res {
                println!("[DEBUG] Blocking task failed: {:?}", e);
                tokio::spawn(async move {
                    let player = Player::new();
                    player.send_event(PlayerEvent::Error(PlayerErrors::UnknownError(e.to_string()))).await;
                });
            }
        });
    }

    pub async fn add_to_queue(&self, track: Track) {
        {
            let mut queue_guard = self.queue.lock().await;
            queue_guard.add_track(track.clone());
            println!("[DEBUG] Added track to queue: title={}", track.title);
        }
        if let Some(tx) = &self.auto_play_tx {
            let _ = tx.send(()).await;
            self.send_event(PlayerEvent::QueueUpdated).await;
        }
    }

    pub async fn play_next(&self) {
        println!("[DEBUG] Playing next track in queue");
        let track_opt = {
            let mut queue = self.queue.lock().await;
            queue.pop_track()
        };
        if let Some(track) = track_opt {
            self.stop().await;
            self.play(track).await;
        } else {
            println!("[DEBUG] Queue is empty");
        }
    }

    pub async fn play_previous(&self) {
        println!("[DEBUG] Playing previous track");
        let track_opt = {
            let mut queue = self.queue.lock().await;
            queue.pop_previous()
        };
        if let Some(track) = track_opt {
            self.stop().await;
            self.play(track).await;
        } else {
            println!("[DEBUG] No previous track available");
        }
    }

    pub async fn pause(&self) {
        println!("[DEBUG] Pausing current playback");
        let mut sink_guard = self.current_sink.lock().await;
        if let Some(sink) = sink_guard.as_mut() {
            sink.pause();
            self.send_event(PlayerEvent::Paused).await;
        } else {
            println!("[DEBUG] No active playback to pause");
            self.send_event(PlayerEvent::Error(PlayerErrors::SinkNotFound)).await;
        }
    }

    pub async fn resume(&self) {
        println!("[DEBUG] Resuming current playback");
        let mut sink_guard = self.current_sink.lock().await;
        if let Some(sink) = sink_guard.as_mut() {
            sink.play();
            self.send_event(PlayerEvent::Resumed).await;
        } else {
            println!("[DEBUG] No active playback to resume");
            self.send_event(PlayerEvent::Error(PlayerErrors::SinkNotFound)).await;
        }
    }

    pub async fn set_volume(&mut self, volume: f32) {
        println!("[DEBUG] Setting volume to: {}", volume);
        {
            let sink_guard = self.current_sink.lock().await;
            if let Some(sink) = sink_guard.as_ref() {
                sink.set_volume(volume);
                self.send_event(PlayerEvent::VolumeChanged(volume)).await;
            } else {
                println!("[DEBUG] No active playback to set volume");
                self.send_event(PlayerEvent::Error(PlayerErrors::SinkNotFound)).await;
            }
        }
        self.volume = volume;
    }

    pub async fn auto_play(&mut self) {
        let (tx, mut rx) = mpsc::channel::<()>(1);
        self.auto_play_tx = Some(tx);

        let player = self.clone();
        let queue = Arc::clone(&self.queue);
        let current_sink = Arc::clone(&self.current_sink);
        let auto_play_enabled = Arc::clone(&self.auto_play_enabled);

        tokio::spawn(async move {
            println!("[DEBUG] Auto-play started");
            player.send_event(PlayerEvent::AutoPlayStarted).await;
            {
                let mut enabled = auto_play_enabled.lock().await;
                *enabled = true;
            }

            loop {
                if !*auto_play_enabled.lock().await {
                    break;
                }

                {
                    let sink_guard = current_sink.lock().await;
                    if sink_guard.is_some() {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                }

                let next_track = {
                    let mut queue_guard = queue.lock().await;
                    queue_guard.pop_track()
                };

                if let Some(track) = next_track {
                    println!("[DEBUG] Playing next track: {}", track.title);
                    player.play(track).await;
                } else {
                    println!("[DEBUG] Queue is empty, waiting for updates...");
                    let _ = rx.recv().await;
                }
            }

            println!("[DEBUG] Auto-play stopped");
        });
    }

    pub async fn is_auto_play_enabled(&self) -> bool {
        *self.auto_play_enabled.lock().await
    }

    pub async fn stop_auto_play(&self) {
        if let Some(tx) = &self.auto_play_tx {
            {
                let mut enabled = self.auto_play_enabled.lock().await;
                *enabled = false;
            }
            let _ = tx.send(()).await;
            self.send_event(PlayerEvent::AutoPlayStopped).await;
        }
    }
}
