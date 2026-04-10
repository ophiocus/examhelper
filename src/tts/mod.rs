mod strip;

pub use strip::{split_into_ranges, strip_markdown, LangRange};

use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicU8, AtomicUsize, Ordering},
    mpsc, Arc, Mutex,
};
use std::thread;

#[derive(Debug, Clone)]
pub struct VoiceInfo {
    pub name: String,
    pub language: String,
}

pub enum TtsCommand {
    Speak(Vec<LangRange>),
    Stop,
    SetRate(f32),
    SetVoice(String),
    SetVoiceMap(HashMap<String, String>),
    Quit,
}

pub fn slider_to_rate(slider: f32) -> f32 {
    let clamped = slider.clamp(0.0, 1.0);
    0.25 + 1.75 * clamped
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NarrationState {
    Idle,
    Speaking,
    Stopped,
    /// Narration completed naturally (not stopped by user).
    Finished,
}

pub struct TtsShared {
    state: AtomicU8,
    current_chunk: AtomicUsize,
    total_chunks: AtomicUsize,
    voices: Mutex<Vec<VoiceInfo>>,
    selected_voice: Mutex<String>,
    narration_ranges: Mutex<Vec<LangRange>>,
}

impl TtsShared {
    fn new() -> Self {
        Self {
            state: AtomicU8::new(0),
            current_chunk: AtomicUsize::new(0),
            total_chunks: AtomicUsize::new(0),
            voices: Mutex::new(Vec::new()),
            selected_voice: Mutex::new(String::new()),
            narration_ranges: Mutex::new(Vec::new()),
        }
    }

    pub fn set_state(&self, s: NarrationState) {
        let v = match s {
            NarrationState::Idle => 0,
            NarrationState::Speaking => 1,
            NarrationState::Stopped => 2,
            NarrationState::Finished => 3,
        };
        self.state.store(v, Ordering::Relaxed);
    }

    pub fn get_state(&self) -> NarrationState {
        match self.state.load(Ordering::Relaxed) {
            1 => NarrationState::Speaking,
            2 => NarrationState::Stopped,
            3 => NarrationState::Finished,
            _ => NarrationState::Idle,
        }
    }
}

/// Controls what happens after narration finishes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlaybackMode {
    /// Manual — narrate only when the user clicks.
    Manual,
    /// AutoRead — narrate automatically when selecting a new section.
    AutoRead,
    /// Loop — repeat the current section when it ends.
    Loop,
    /// Continue — advance to the next section and narrate it.
    Continue,
}

impl PlaybackMode {
    pub fn next(self) -> Self {
        match self {
            Self::Manual => Self::AutoRead,
            Self::AutoRead => Self::Loop,
            Self::Loop => Self::Continue,
            Self::Continue => Self::Manual,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Manual => "Manual",
            Self::AutoRead => "Auto",
            Self::Loop => "Loop",
            Self::Continue => "Next",
        }
    }

    pub fn auto_narrate_on_select(self) -> bool {
        matches!(self, Self::AutoRead | Self::Continue)
    }
}

pub struct TtsController {
    tx: mpsc::Sender<TtsCommand>,
    shared: Arc<TtsShared>,
    pub playback_mode: PlaybackMode,
    default_lang: Mutex<String>,
}

impl TtsController {
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel::<TtsCommand>();
        let shared = Arc::new(TtsShared::new());
        let shared_clone = Arc::clone(&shared);

        thread::Builder::new()
            .name("tts-worker".to_string())
            .stack_size(8 * 1024 * 1024)
            .spawn(move || {
                Self::worker(rx, shared_clone);
            })
            .expect("failed to spawn TTS thread");

        Self {
            tx,
            shared,
            playback_mode: PlaybackMode::Manual,
            default_lang: Mutex::new("en".to_string()),
        }
    }

    fn worker(rx: mpsc::Receiver<TtsCommand>, shared: Arc<TtsShared>) {
        let mut tts = match tts::Tts::default() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("TTS init failed: {e}");
                return;
            }
        };

        let mut all_voices: Vec<tts::Voice> = Vec::new();
        if let Ok(voices) = tts.voices() {
            {
                let mut voice_list = shared.voices.lock().unwrap();
                for v in &voices {
                    voice_list.push(VoiceInfo {
                        name: v.name().to_string(),
                        language: v.language().to_string(),
                    });
                }
            }
            all_voices = voices;

            let default_voice = all_voices
                .iter()
                .find(|v| v.language().to_string().to_lowercase().starts_with("en-us"))
                .or_else(|| {
                    all_voices
                        .iter()
                        .find(|v| v.language().to_string().to_lowercase().starts_with("en"))
                })
                .or(all_voices.first());

            if let Some(voice) = default_voice {
                let _ = tts.set_voice(voice);
                if let Ok(mut sel) = shared.selected_voice.lock() {
                    *sel = voice.name().to_string();
                }
            }
        }

        let normal = tts.normal_rate();
        let _ = tts.set_rate(normal);

        let mut voice_map: HashMap<String, String> = HashMap::new();

        loop {
            match rx.recv() {
                Ok(TtsCommand::Speak(initial_ranges)) => {
                    let mut ranges = initial_ranges;
                    'narrate: loop {
                        // One speak() call per range — no intra-range snaps.
                        // Progress tracks ranges, not chunks.
                        shared.total_chunks.store(ranges.len(), Ordering::Relaxed);
                        shared.current_chunk.store(0, Ordering::Relaxed);
                        shared.set_state(NarrationState::Speaking);
                        if let Ok(mut nr) = shared.narration_ranges.lock() {
                            *nr = ranges.clone();
                        }

                        let mut restart_with: Option<Vec<LangRange>> = None;

                        for (range_idx, range) in ranges.iter().enumerate() {
                            shared.current_chunk.store(range_idx, Ordering::Relaxed);

                            // Switch voice once per range
                            switch_voice_for_lang(
                                &range.lang,
                                &mut tts,
                                &all_voices,
                                &voice_map,
                            );

                            // Join all chunks into a single utterance — one speak()
                            // call means zero audio snaps within the range.
                            let full_text = range.chunks.join("\n\n");
                            let _ = tts.speak(&full_text, false);

                            // Poll until this single utterance finishes
                            while tts.is_speaking().unwrap_or(false) {
                                thread::sleep(std::time::Duration::from_millis(50));
                                if let Ok(cmd) = rx.try_recv() {
                                    match cmd {
                                        TtsCommand::Stop => {
                                            let _ = tts.stop();
                                            shared.set_state(NarrationState::Stopped);
                                            break;
                                        }
                                        TtsCommand::Speak(new) => {
                                            let _ = tts.stop();
                                            restart_with = Some(new);
                                            break;
                                        }
                                        TtsCommand::SetRate(r) => {
                                            let _ = tts.set_rate(slider_to_rate(r));
                                        }
                                        TtsCommand::SetVoice(name) => {
                                            set_voice_by_name(
                                                &name,
                                                &mut tts,
                                                &all_voices,
                                                &shared,
                                            );
                                        }
                                        TtsCommand::SetVoiceMap(map) => {
                                            voice_map = map;
                                        }
                                        TtsCommand::Quit => {
                                            let _ = tts.stop();
                                            return;
                                        }
                                    }
                                }
                            }

                            if shared.get_state() == NarrationState::Stopped
                                || restart_with.is_some()
                            {
                                break;
                            }
                        }

                        match restart_with {
                            Some(new) => {
                                ranges = new;
                                continue 'narrate;
                            }
                            None => {
                                if shared.get_state() == NarrationState::Speaking {
                                    shared.set_state(NarrationState::Finished);
                                }
                                break 'narrate;
                            }
                        }
                    }
                }
                Ok(TtsCommand::Stop) => {
                    shared.set_state(NarrationState::Idle);
                }
                Ok(TtsCommand::SetRate(r)) => {
                    let _ = tts.set_rate(slider_to_rate(r));
                }
                Ok(TtsCommand::SetVoice(name)) => {
                    set_voice_by_name(&name, &mut tts, &all_voices, &shared);
                }
                Ok(TtsCommand::SetVoiceMap(map)) => {
                    voice_map = map;
                }
                Ok(TtsCommand::Quit) | Err(_) => {
                    let _ = tts.stop();
                    return;
                }
            }
        }
    }

    pub fn speak(&self, text: &str) {
        let cleaned = strip_markdown(text);
        let default_lang = self.default_lang.lock().unwrap().clone();
        let ranges = split_into_ranges(&cleaned, &default_lang);
        if let Ok(mut nr) = self.shared.narration_ranges.lock() {
            *nr = ranges.clone();
        }
        let _ = self.tx.send(TtsCommand::Speak(ranges));
    }

    pub fn stop(&self) {
        let _ = self.tx.send(TtsCommand::Stop);
    }

    pub fn set_rate(&self, rate: f32) {
        let _ = self.tx.send(TtsCommand::SetRate(rate));
    }

    pub fn set_voice(&self, name: &str) {
        let _ = self.tx.send(TtsCommand::SetVoice(name.to_string()));
    }

    pub fn set_voice_map(&self, map: HashMap<String, String>) {
        let _ = self.tx.send(TtsCommand::SetVoiceMap(map));
    }

    pub fn set_default_lang(&self, lang: &str) {
        if let Ok(mut dl) = self.default_lang.lock() {
            *dl = lang.to_string();
        }
    }

    pub fn state(&self) -> NarrationState {
        self.shared.get_state()
    }

    /// Check if narration just finished naturally. Consumes the state (sets to Idle).
    pub fn take_finished(&self) -> bool {
        if self.shared.get_state() == NarrationState::Finished {
            self.shared.set_state(NarrationState::Idle);
            true
        } else {
            false
        }
    }

    pub fn progress(&self) -> (usize, usize) {
        (
            self.shared.current_chunk.load(Ordering::Relaxed),
            self.shared.total_chunks.load(Ordering::Relaxed),
        )
    }

    pub fn voices(&self) -> Vec<VoiceInfo> {
        self.shared.voices.lock().unwrap().clone()
    }

    pub fn selected_voice_name(&self) -> String {
        self.shared.selected_voice.lock().unwrap().clone()
    }

    /// Return ranges for karaoke display — one entry per range (full text + lang).
    pub fn narration_chunks_flat(&self) -> Vec<(String, String)> {
        let ranges = self.shared.narration_ranges.lock().unwrap().clone();
        ranges
            .iter()
            .map(|r| (r.chunks.join("\n\n"), r.lang.clone()))
            .collect()
    }
}

impl Drop for TtsController {
    fn drop(&mut self) {
        let _ = self.tx.send(TtsCommand::Quit);
    }
}

/// Switch to the best voice for a language, using the voice map or prefix matching.
fn switch_voice_for_lang(
    lang: &str,
    tts: &mut tts::Tts,
    all_voices: &[tts::Voice],
    voice_map: &HashMap<String, String>,
) {
    if let Some(voice_name) = voice_map.get(lang) {
        if let Some(v) = all_voices.iter().find(|v| v.name() == voice_name.as_str()) {
            let _ = tts.set_voice(v);
            return;
        }
    }
    // Fallback: prefix match
    let ll = lang.to_lowercase();
    if let Some(v) = all_voices
        .iter()
        .find(|v| v.language().to_string().to_lowercase().starts_with(&ll))
    {
        let _ = tts.set_voice(v);
    }
}

/// Set voice by name and update shared state.
fn set_voice_by_name(
    name: &str,
    tts: &mut tts::Tts,
    all_voices: &[tts::Voice],
    shared: &Arc<TtsShared>,
) {
    if let Some(v) = all_voices.iter().find(|v| v.name() == name) {
        let _ = tts.set_voice(v);
        if let Ok(mut sel) = shared.selected_voice.lock() {
            *sel = name.to_string();
        }
    }
}
