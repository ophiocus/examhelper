mod strip;

pub use strip::{split_into_lang_chunks, strip_lang_tags, strip_markdown, LangChunk};

use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicU8, AtomicUsize, Ordering},
    mpsc, Arc, Mutex,
};
use std::thread;

/// A voice entry for the GUI dropdown.
#[derive(Debug, Clone)]
pub struct VoiceInfo {
    pub name: String,
    pub language: String,
}

/// Commands sent from the GUI thread to the TTS worker thread.
pub enum TtsCommand {
    Speak(Vec<LangChunk>),
    Stop,
    SetRate(f32),
    SetVoice(String),
    SetVoiceMap(HashMap<String, String>),
    Quit,
}

/// Map a 0.0-1.0 slider value to WinRT speech rate.
pub fn slider_to_rate(slider: f32) -> f32 {
    let clamped = slider.clamp(0.0, 1.0);
    0.25 + 1.75 * clamped
}

/// Narration state visible to the GUI.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NarrationState {
    Idle,
    Speaking,
    Stopped,
}

/// Shared state between GUI and TTS worker.
pub struct TtsShared {
    state: AtomicU8,
    current_chunk: AtomicUsize,
    total_chunks: AtomicUsize,
    voices: Mutex<Vec<VoiceInfo>>,
    selected_voice: Mutex<String>,
    narration_chunks: Mutex<Vec<LangChunk>>,
}

impl TtsShared {
    fn new() -> Self {
        Self {
            state: AtomicU8::new(0),
            current_chunk: AtomicUsize::new(0),
            total_chunks: AtomicUsize::new(0),
            voices: Mutex::new(Vec::new()),
            selected_voice: Mutex::new(String::new()),
            narration_chunks: Mutex::new(Vec::new()),
        }
    }

    pub fn set_state(&self, s: NarrationState) {
        let v = match s {
            NarrationState::Idle => 0,
            NarrationState::Speaking => 1,
            NarrationState::Stopped => 2,
        };
        self.state.store(v, Ordering::Relaxed);
    }

    pub fn get_state(&self) -> NarrationState {
        match self.state.load(Ordering::Relaxed) {
            1 => NarrationState::Speaking,
            2 => NarrationState::Stopped,
            _ => NarrationState::Idle,
        }
    }
}

/// Controller handle held by the GUI.
pub struct TtsController {
    tx: mpsc::Sender<TtsCommand>,
    shared: Arc<TtsShared>,
    pub autoplay: bool,
    default_lang: Mutex<String>,
}

impl TtsController {
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel::<TtsCommand>();
        let shared = Arc::new(TtsShared::new());
        let shared_clone = Arc::clone(&shared);

        thread::Builder::new()
            .name("tts-worker".to_string())
            .stack_size(4 * 1024 * 1024)
            .spawn(move || {
                Self::worker(rx, shared_clone);
            })
            .expect("failed to spawn TTS thread");

        Self {
            tx,
            shared,
            autoplay: false,
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
                .find(|v| {
                    v.language()
                        .to_string()
                        .to_lowercase()
                        .starts_with("en-us")
                })
                .or_else(|| {
                    all_voices.iter().find(|v| {
                        v.language()
                            .to_string()
                            .to_lowercase()
                            .starts_with("en")
                    })
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
        let mut current_voice_lang = String::new();

        // Main command loop — kept flat to minimize stack depth
        loop {
            match rx.recv() {
                Ok(TtsCommand::Speak(initial_chunks)) => {
                    // Narration loop: handles the current chunks, and if a new
                    // Speak arrives mid-narration, swaps in the new chunks and
                    // restarts — all without adding stack frames.
                    let mut chunks = initial_chunks;
                    'narrate: loop {
                        let total = chunks.len();
                        shared.total_chunks.store(total, Ordering::Relaxed);
                        shared.current_chunk.store(0, Ordering::Relaxed);
                        shared.set_state(NarrationState::Speaking);
                        if let Ok(mut nc) = shared.narration_chunks.lock() {
                            *nc = chunks.clone();
                        }

                        let mut restart_with: Option<Vec<LangChunk>> = None;

                        for (i, chunk) in chunks.iter().enumerate() {
                            // Check for commands between chunks
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
                                        if let Some(v) =
                                            all_voices.iter().find(|v| v.name() == name)
                                        {
                                            let _ = tts.set_voice(v);
                                            if let Ok(mut sel) = shared.selected_voice.lock() {
                                                *sel = name;
                                            }
                                            current_voice_lang.clear();
                                        }
                                    }
                                    TtsCommand::SetVoiceMap(map) => {
                                        voice_map = map;
                                    }
                                    TtsCommand::Quit => {
                                        let _ = tts.stop();
                                        return;
                                    }
                                }
                                if shared.get_state() == NarrationState::Stopped {
                                    break;
                                }
                            }

                            // Switch voice if chunk language differs
                            if chunk.lang != current_voice_lang {
                                let voice_name = voice_map.get(&chunk.lang);
                                let found = voice_name.and_then(|vn| {
                                    all_voices.iter().find(|v| v.name() == vn.as_str())
                                });
                                let found = found.or_else(|| {
                                    let ll = chunk.lang.to_lowercase();
                                    all_voices.iter().find(|v| {
                                        v.language()
                                            .to_string()
                                            .to_lowercase()
                                            .starts_with(&ll)
                                    })
                                });
                                if let Some(v) = found {
                                    let _ = tts.set_voice(v);
                                    current_voice_lang = chunk.lang.clone();
                                }
                            }

                            shared.current_chunk.store(i, Ordering::Relaxed);
                            let _ = tts.speak(&chunk.text, false);

                            // Wait for utterance to finish, checking for commands
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
                                            if let Some(v) =
                                                all_voices.iter().find(|v| v.name() == name)
                                            {
                                                let _ = tts.set_voice(v);
                                                if let Ok(mut sel) =
                                                    shared.selected_voice.lock()
                                                {
                                                    *sel = name;
                                                }
                                                current_voice_lang.clear();
                                            }
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

                        // If a new Speak arrived, swap chunks and re-enter narrate loop
                        match restart_with {
                            Some(new) => {
                                chunks = new;
                                continue 'narrate;
                            }
                            None => {
                                if shared.get_state() == NarrationState::Speaking {
                                    shared.set_state(NarrationState::Idle);
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
                    if let Some(voice) = all_voices.iter().find(|v| v.name() == name) {
                        let _ = tts.set_voice(voice);
                        if let Ok(mut sel) = shared.selected_voice.lock() {
                            *sel = name;
                        }
                        current_voice_lang.clear();
                    }
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
        let chunks = split_into_lang_chunks(&cleaned, &default_lang);
        if let Ok(mut nc) = self.shared.narration_chunks.lock() {
            *nc = chunks.clone();
        }
        let _ = self.tx.send(TtsCommand::Speak(chunks));
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

    pub fn narration_chunks(&self) -> Vec<LangChunk> {
        self.shared.narration_chunks.lock().unwrap().clone()
    }
}

impl Drop for TtsController {
    fn drop(&mut self) {
        let _ = self.tx.send(TtsCommand::Quit);
    }
}
