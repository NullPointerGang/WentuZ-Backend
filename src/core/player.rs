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
use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;
use std::sync::Arc;

use tokio::sync::{Mutex, mpsc};

#[derive(Clone)]
pub struct Player {
    queue: Arc<Mutex<Queue>>,
    current_sink: Arc<Mutex<Option<Arc<Sink>>>>,
    auto_play_enabled: Arc<Mutex<bool>>,
    auto_play_tx: Option<mpsc::Sender<()>>,
    volume: f32,
}

impl Player {
    pub fn new() -> Self {
        println!("[DEBUG] Initializing player");
        Player {
            queue: Arc::new(Mutex::new(Queue::new())),
            current_sink: Arc::new(Mutex::new(None)),
            auto_play_enabled: Arc::new(Mutex::new(false)),
            auto_play_tx: None,
            volume: 1.0,
        }
    }

    pub async fn seek(&self, position: Duration) {
        println!("[DEBUG] Seeking to position: {:?}", position);
        let sink_guard = self.current_sink.lock().await;
        if let Some(sink) = sink_guard.as_ref() {
            if let Err(e) = sink.try_seek(position) {
                println!("[DEBUG] Failed to seek to position: {:?}, error: {:?}", position, e);
            }
        } else {
            println!("[DEBUG] Нет активного воспроизведения для перемотки");
        }
    }

    pub async fn stop(&self) {
        println!("[DEBUG] Stopping current playback");
        let mut sink_guard = self.current_sink.lock().await;
        if let Some(sink) = sink_guard.take() {
            sink.stop();
        } else {
            println!("[DEBUG] No active playback to stop");
        }
    }

    pub async fn play(&self, track: Track) {
        // Останавливаем предыдущее воспроизведение
        self.stop().await;

        println!(
            "[DEBUG] Starting playback in an async task for track: title={} file_path={:?}",
            track.title, track.file_path
        );

        let track_data = track.file_data.clone();
        let track_title = track.title.clone();
        let current_sink = Arc::clone(&self.current_sink);
        let volume = self.volume;

        // Запускаем задачу для воспроизведения
        tokio::spawn(async move {
            let res = tokio::task::spawn_blocking(move || {
                let cursor = Cursor::new(track_data);
                match Decoder::new_looped(cursor) {
                    Ok(source) => {
                        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
                        let sink = Arc::new(Sink::try_new(&stream_handle).unwrap());
                        sink.set_volume(volume);

                        // Обновляем текущий sink (блокирующе, т.к. внутри spawn_blocking)
                        {
                            let mut sink_guard = current_sink.blocking_lock();
                            *sink_guard = Some(Arc::clone(&sink));
                        }

                        sink.append(source);

                        println!("[DEBUG] Playback started for track: title={}", track_title);

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
            }
        });
    }

    pub async fn add_to_queue(&self, track: Track) {
        {
            let mut queue_guard = self.queue.lock().await;
            queue_guard.add_track(track);
        }
        if let Some(tx) = &self.auto_play_tx {
            let _ = tx.send(()).await;
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
        } else {
            println!("[DEBUG] No active playback to pause");
        }
    }

    pub async fn resume(&self) {
        println!("[DEBUG] Resuming current playback");
        let mut sink_guard = self.current_sink.lock().await;
        if let Some(sink) = sink_guard.as_mut() {
            sink.play();
        } else {
            println!("[DEBUG] No active playback to resume");
        }
    }

    pub async fn set_volume(&mut self, volume: f32) {
        println!("[DEBUG] Setting volume to: {}", volume);
        {
            let sink_guard = self.current_sink.lock().await;
            if let Some(sink) = sink_guard.as_ref() {
                sink.set_volume(volume);
            } else {
                println!("[DEBUG] No active playback to set volume");
            }
        }
        self.volume = volume;
    }

    pub async fn auto_play(&mut self) {
        // Создаем асинхронный канал для уведомлений об обновлении очереди
        let (tx, mut rx) = mpsc::channel::<()>(1);
        self.auto_play_tx = Some(tx);

        // Для вызова методов плеера из задачи необходимо клонировать self
        let player = self.clone();
        let queue = Arc::clone(&self.queue);
        let current_sink = Arc::clone(&self.current_sink);
        let auto_play_enabled = Arc::clone(&self.auto_play_enabled);

        tokio::spawn(async move {
            println!("[DEBUG] Auto-play started");
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
                    // Ожидаем сигнал об обновлении очереди
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
        }
    }

    pub fn get_queue(&self) -> Arc<Mutex<Queue>> {
        Arc::clone(&self.queue)
    }
}