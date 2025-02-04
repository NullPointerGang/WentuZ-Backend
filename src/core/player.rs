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
 use std::sync::{Arc, Mutex};
 use std::sync::mpsc::{self, Sender};
 use std::thread;
 
 
 #[derive(Clone)]
 pub struct Player {
     queue: Arc<Mutex<Queue>>,
     current_sink: Arc<Mutex<Option<Arc<Sink>>>>,
     auto_play_enabled: Arc<Mutex<bool>>,
     auto_play_tx: Option<Sender<()>>,
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
 
     pub fn seek(&self, position: Duration) {
         println!("[DEBUG] Seeking to position: {:?}", position);
         
         let sink_guard = self.current_sink.lock().unwrap();
         if let Some(sink) = sink_guard.as_ref() {
             if let Err(e) = sink.try_seek(position) {
                 println!("[DEBUG] Failed to seek to position: {:?}, error: {:?}", position, e);
             }
         } else {
             println!("[DEBUG] Нет активного воспроизведения для перемотки");
         }
     }
 
     pub fn stop(&self) {
         println!("[DEBUG] Stopping current playback");
         let mut sink_guard = self.current_sink.lock().unwrap();
         if let Some(sink) = sink_guard.take() {
             sink.stop();
         } else {
             println!("[DEBUG] No active playback to stop");
         }
     }
 
     pub fn play(&self, track: Track) {
         self.stop();
     
         println!("[DEBUG] Starting playback in a new thread for track: title={} file_path={:?}",
             track.title, track.file_path
         );
     
         let track_data = track.file_data.clone();
         let track_title = track.title.clone();
         let current_sink = Arc::clone(&self.current_sink);
         
         let volume = self.volume;
 
         thread::spawn(move || {
             let cursor = Cursor::new(track_data);
             if let Ok(source) = Decoder::new_looped(cursor) {
                 let (_stream, stream_handle) = OutputStream::try_default().unwrap();
                 let sink = Arc::new(Sink::try_new(&stream_handle).unwrap());
 
                 sink.set_volume(volume);
                 {
                     let mut sink_guard = current_sink.lock().unwrap();
                     *sink_guard = Some(Arc::clone(&sink));
                 }
     
                 sink.append(source);
     
                 println!("[DEBUG] Playback started for track: title={}", track_title);
                 
                 sink.sleep_until_end();
     
                 println!("[DEBUG] Playback completed for track: title={}", track_title);
                 let mut sink_guard = current_sink.lock().unwrap();
                 *sink_guard = None;
             } else {
                 println!("[DEBUG] Failed to decode track: title={}", track_title);
             }
         });
     }
     
 
     pub fn add_to_queue(&self, track: Track) {
         {
             let mut queue_guard = self.queue.lock().unwrap();
             queue_guard.add_track(track);
         }
 
         if let Some(tx) = &self.auto_play_tx {
             let _ = tx.send(());
         }
     }
 
     pub fn play_next(&self) {
         println!("[DEBUG] Playing next track in queue");
         let mut queue = self.queue.lock().unwrap(); 
         if let Some(track) = queue.pop_track() {
             self.stop();
             self.play(track);
         } else {
             println!("[DEBUG] Queue is empty");
         }
     }
 
     pub fn play_previous(&self) {
         println!("[DEBUG] Playing previous track");
         let mut queue = self.queue.lock().unwrap();
         if let Some(track) = queue.pop_previous() {
             self.stop();
             self.play(track);
         } else {
             println!("[DEBUG] No previous track available");
         }
     }
 
     pub fn pause(&self) {
         println!("[DEBUG] Pausing playback");
         let sink_guard = self.current_sink.lock().unwrap();
         if let Some(sink) = sink_guard.as_ref() {
             sink.pause();
         } else {
             println!("[DEBUG] No active playback to pause");
         }
     }
 
     pub fn resume(&self) {
         println!("[DEBUG] Resuming playback");
         let sink_guard = self.current_sink.lock().unwrap();
         if let Some(sink) = sink_guard.as_ref() {
             sink.play();
         } else {
             println!("[DEBUG] No active playback to resume");
         }
     }
 
     pub fn set_volume(&mut self, volume: f32) {
         println!("[DEBUG] Setting volume to: {}", volume);
         let sink_guard = self.current_sink.lock().unwrap();
         if let Some(sink) = sink_guard.as_ref() {
             sink.set_volume(volume);
         } else {
             println!("[DEBUG] No active playback to set volume");
         }
         self.volume = volume;
     }
 
     pub fn auto_play(&mut self) {
         let (tx, rx) = mpsc::channel();
         self.auto_play_tx = Some(tx.clone());
 
         
         let player = Arc::new(Mutex::new(self.clone()));
 
         let queue = Arc::clone(&self.queue);
         let current_sink = Arc::clone(&self.current_sink);
         let auto_play_enabled = Arc::clone(&self.auto_play_enabled);
 
         thread::spawn(move || {
             println!("[DEBUG] Auto-play started");
             *auto_play_enabled.lock().unwrap() = true;
 
             loop {
                 if !*auto_play_enabled.lock().unwrap() {
                     break;
                 }
 
                 {
                     let sink_guard = current_sink.lock().unwrap();
                     if sink_guard.is_some() {
                         thread::sleep(Duration::from_millis(100));
                         continue;
                     }
                 }
 
                 let next_track = {
                     let mut queue_guard = queue.lock().unwrap();
                     queue_guard.pop_track()
                 };
 
                 if let Some(track) = next_track {
                     println!("[DEBUG] Playing next track: {}", track.title);
                     let player_lock = player.lock().unwrap();
                     player_lock.play(track);
                 } else {
                     println!("[DEBUG] Queue is empty, waiting for updates...");
                     if rx.recv_timeout(Duration::from_secs(10)).is_err() {
                         println!("[DEBUG] No updates received within 10 seconds");
                     }
                 }
             }
 
             println!("[DEBUG] Auto-play stopped");
         });
     }
 
     pub fn is_auto_play_enabled(&self) -> bool {
         *self.auto_play_enabled.lock().unwrap()
     }
 
     pub fn stop_auto_play(&self) {
         if let Some(tx) = &self.auto_play_tx {
             *self.auto_play_enabled.lock().unwrap() = false;
             let _ = tx.send(());
         }
     }
 
     pub fn get_queue(&self) -> Arc<Mutex<Queue>> {
         Arc::clone(&self.queue)
     }
 }