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


use crate::core::track::Track;
use rand::seq::SliceRandom;


#[derive(Clone)]
pub struct Queue {
    tracks: Vec<Track>,
    played_tracks: Vec<Track>,
}

impl Queue {
    pub fn new() -> Self {
        println!("[DEBUG] Initializing empty queue");
        Queue {
            tracks: Vec::new(),
            played_tracks: Vec::new(),
        }
    }

    pub fn add_track(&mut self, track: Track) {
        println!("[DEBUG] Adding track to queue: title={}", track.title);
        self.tracks.push(track);
    }

    pub fn pop_track(&mut self) -> Option<Track> {
        if let Some(track) = self.tracks.first().cloned() {
            println!("[DEBUG] Removing track from queue: title={}", track.title);
            self.tracks.remove(0);
            self.played_tracks.push(track.clone());
            Some(track)
        } else {
            println!("[DEBUG] Attempted to pop track from empty queue");
            None
        }
    }

    pub fn peek(&self) -> Option<&Track> {
        if let Some(track) = self.tracks.first() {
            println!("[DEBUG] Peeking track: title={}", track.title);
            Some(track)
        } else {
            println!("[DEBUG] Peeked at empty queue");
            None
        }
    }

    pub fn shuffle(&mut self) {
        println!("[DEBUG] Shuffling queue with {} tracks", self.tracks.len());
        let mut rng = rand::rng();
        self.tracks.shuffle(&mut rng);
    }

    pub fn skip(&mut self) {
        println!("[DEBUG] Skipping current track");
        self.pop_track();
    }

    pub fn clear(&mut self) {
        println!("[DEBUG] Clearing queue with {} tracks", self.tracks.len());
        self.tracks.clear();
        self.played_tracks.clear();
    }

    pub fn pop_previous(&mut self) -> Option<Track> {
        if let Some(track) = self.played_tracks.pop() {
            println!("[DEBUG] Returning to previous track: title={}", track.title);
            Some(track)
        } else {
            println!("[DEBUG] No previous track to return to");
            None
        }
    }

    pub fn get_tracks(&self) -> &Vec<Track> {
        &self.tracks
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    pub fn queue_iter(&self) -> impl Iterator<Item = &Track> {
        self.tracks.iter()
    }
}