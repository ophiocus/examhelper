use std::io::BufReader;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use rodio::{Decoder, OutputStream, Sink};

use super::{NarrationState, TtsCommand, TtsShared};

/// Play a sequence of cached WAV files with chunk-level progress tracking.
/// Returns true if playback completed naturally, false if stopped/restarted.
pub fn play_cached(
    wav_paths: &[PathBuf],
    shared: &Arc<TtsShared>,
    rx: &mpsc::Receiver<TtsCommand>,
) -> Option<Vec<super::LangRange>> {
    let total = wav_paths.len();
    shared.total_chunks.store(total, Ordering::Relaxed);
    shared.current_chunk.store(0, Ordering::Relaxed);
    shared.set_state(NarrationState::Speaking);

    // Open audio output once for the entire playback session
    let (_stream, stream_handle) = match OutputStream::try_default() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Audio output failed: {e}");
            shared.set_state(NarrationState::Stopped);
            return None;
        }
    };

    for (idx, wav_path) in wav_paths.iter().enumerate() {
        // Check for commands before playing
        if let Ok(cmd) = rx.try_recv() {
            match cmd {
                TtsCommand::Stop => {
                    shared.set_state(NarrationState::Stopped);
                    return None;
                }
                TtsCommand::Speak(new) => {
                    return Some(new);
                }
                _ => {} // rate/voice changes don't apply to cached playback
            }
        }

        shared.current_chunk.store(idx, Ordering::Relaxed);

        // Load and play WAV
        let file = match std::fs::File::open(wav_path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Failed to open WAV {}: {e}", wav_path.display());
                continue;
            }
        };
        let source = match Decoder::new(BufReader::new(file)) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to decode WAV {}: {e}", wav_path.display());
                continue;
            }
        };

        let sink = match Sink::try_new(&stream_handle) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to create sink: {e}");
                continue;
            }
        };
        sink.append(source);

        // Poll until this chunk finishes
        while !sink.empty() {
            thread::sleep(Duration::from_millis(50));
            if let Ok(cmd) = rx.try_recv() {
                match cmd {
                    TtsCommand::Stop => {
                        sink.stop();
                        shared.set_state(NarrationState::Stopped);
                        return None;
                    }
                    TtsCommand::Speak(new) => {
                        sink.stop();
                        return Some(new);
                    }
                    _ => {}
                }
            }
        }

        if shared.get_state() == NarrationState::Stopped {
            return None;
        }
    }

    // Completed naturally
    shared.set_state(NarrationState::Finished);
    None
}
