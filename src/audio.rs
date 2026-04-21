//! Embedded-asset audio playback for short UI sounds.
//!
//! A single lazily initialised `MixerDeviceSink` is held for the lifetime of
//! the process; individual sound plays are cheap and non-blocking. If audio
//! fails to initialise (headless CI, no audio device, etc.) the calls become
//! silent no-ops.

use std::io::Cursor;
use std::sync::{Mutex, OnceLock};

use rodio::stream::{DeviceSinkBuilder, MixerDeviceSink};

use crate::assets::Assets;

static SINK: OnceLock<Mutex<Option<MixerDeviceSink>>> = OnceLock::new();

fn sink() -> &'static Mutex<Option<MixerDeviceSink>> {
    SINK.get_or_init(|| {
        let sink = match DeviceSinkBuilder::open_default_sink() {
            Ok(mut s) => {
                s.log_on_drop(false);
                Some(s)
            }
            Err(e) => {
                eprintln!("bmo: audio unavailable ({e})");
                None
            }
        };
        Mutex::new(sink)
    })
}

fn play_embedded(path: &str) {
    let Some(file) = Assets::get(path) else {
        eprintln!("bmo: missing sound asset {path}");
        return;
    };

    let guard = sink().lock().ok();
    let Some(mut guard) = guard else {
        return;
    };
    let Some(sink) = guard.as_mut() else {
        return;
    };

    // `rodio::play` takes any `Read + Seek`. We copy the bytes into a Cursor
    // so the stream owns its data and can outlive this call.
    let bytes: Vec<u8> = file.data.into_owned();
    let cursor = Cursor::new(bytes);

    match rodio::play(sink.mixer(), cursor) {
        Ok(player) => {
            // Detach: the Player will free itself once the sample completes.
            std::mem::forget(player);
        }
        Err(e) => eprintln!("bmo: failed to play {path}: {e}"),
    }
}

pub struct AudioPlayer;

impl AudioPlayer {
    /// Short ding for segment boundaries.
    pub fn play_ding() {
        play_embedded("sounds/ding.wav");
    }

    /// Longer chime for end-of-preset.
    pub fn play_complete() {
        play_embedded("sounds/complete.wav");
    }
}
