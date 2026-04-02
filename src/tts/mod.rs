mod strip;

pub use strip::{split_into_chunks, strip_markdown};

use std::sync::{
    atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
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
    Speak(String),
    Stop,
    SetRate(f32),
    SetVoice(String),
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
    narration_chunks: Mutex<Vec<String>>,
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
}

impl TtsController {
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel::<TtsCommand>();
        let shared = Arc::new(TtsShared::new());
        let shared_clone = Arc::clone(&shared);

        thread::spawn(move || {
            Self::worker(rx, shared_clone);
        });

        Self {
            tx,
            shared,
            autoplay: false,
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

            let spanish_voice = all_voices
                .iter()
                .find(|v| {
                    v.language()
                        .to_string()
                        .to_lowercase()
                        .starts_with("es-co")
                })
                .or_else(|| {
                    all_voices.iter().find(|v| {
                        v.language()
                            .to_string()
                            .to_lowercase()
                            .starts_with("es-mx")
                    })
                })
                .or_else(|| {
                    all_voices.iter().find(|v| {
                        v.language()
                            .to_string()
                            .to_lowercase()
                            .starts_with("es")
                    })
                });

            if let Some(voice) = spanish_voice {
                let _ = tts.set_voice(voice);
                if let Ok(mut sel) = shared.selected_voice.lock() {
                    *sel = voice.name().to_string();
                }
            } else if let Some(first) = all_voices.first() {
                if let Ok(mut sel) = shared.selected_voice.lock() {
                    *sel = first.name().to_string();
                }
            }
        }

        let normal = tts.normal_rate();
        let _ = tts.set_rate(normal);

        let stop_flag = Arc::new(AtomicBool::new(false));

        loop {
            match rx.recv() {
                Ok(TtsCommand::Speak(text)) => {
                    stop_flag.store(false, Ordering::Relaxed);

                    let chunks = split_into_chunks(&text);
                    let total = chunks.len();
                    shared.total_chunks.store(total, Ordering::Relaxed);
                    shared.current_chunk.store(0, Ordering::Relaxed);
                    shared.set_state(NarrationState::Speaking);

                    for (i, chunk) in chunks.iter().enumerate() {
                        if stop_flag.load(Ordering::Relaxed) {
                            break;
                        }
                        if let Ok(cmd) = rx.try_recv() {
                            match cmd {
                                TtsCommand::Stop => {
                                    let _ = tts.stop();
                                    shared.set_state(NarrationState::Stopped);
                                    break;
                                }
                                TtsCommand::Speak(new_text) => {
                                    let _ = tts.stop();
                                    shared.set_state(NarrationState::Idle);
                                    let chunks2 = split_into_chunks(&new_text);
                                    let total2 = chunks2.len();
                                    shared.total_chunks.store(total2, Ordering::Relaxed);
                                    shared.current_chunk.store(0, Ordering::Relaxed);
                                    shared.set_state(NarrationState::Speaking);
                                    // Store new chunks for karaoke
                                    if let Ok(mut nc) = shared.narration_chunks.lock() {
                                        *nc = chunks2.clone();
                                    }
                                    for (j, c2) in chunks2.iter().enumerate() {
                                        if let Ok(cmd2) = rx.try_recv() {
                                            match cmd2 {
                                                TtsCommand::Stop => {
                                                    let _ = tts.stop();
                                                    shared.set_state(NarrationState::Stopped);
                                                    break;
                                                }
                                                TtsCommand::SetRate(r) => {
                                                    let _ = tts.set_rate(slider_to_rate(r));
                                                }
                                                TtsCommand::Quit => {
                                                    let _ = tts.stop();
                                                    return;
                                                }
                                                _ => {}
                                            }
                                        }
                                        shared.current_chunk.store(j, Ordering::Relaxed);
                                        let _ = tts.speak(c2, false);
                                        while tts.is_speaking().unwrap_or(false) {
                                            thread::sleep(std::time::Duration::from_millis(50));
                                            if let Ok(cmd2) = rx.try_recv() {
                                                match cmd2 {
                                                    TtsCommand::Stop => {
                                                        let _ = tts.stop();
                                                        shared.set_state(NarrationState::Stopped);
                                                        break;
                                                    }
                                                    TtsCommand::SetRate(r) => {
                                                        let _ = tts.set_rate(slider_to_rate(r));
                                                    }
                                                    _ => {}
                                                }
                                            }
                                        }
                                        if shared.get_state() == NarrationState::Stopped {
                                            break;
                                        }
                                    }
                                    if shared.get_state() == NarrationState::Speaking {
                                        shared.set_state(NarrationState::Idle);
                                    }
                                    break;
                                }
                                TtsCommand::SetRate(r) => {
                                    let _ = tts.set_rate(slider_to_rate(r));
                                }
                                TtsCommand::SetVoice(name) => {
                                    if let Some(voice) =
                                        all_voices.iter().find(|v| v.name() == name)
                                    {
                                        let _ = tts.set_voice(voice);
                                        if let Ok(mut sel) = shared.selected_voice.lock() {
                                            *sel = name;
                                        }
                                    }
                                }
                                TtsCommand::Quit => {
                                    let _ = tts.stop();
                                    return;
                                }
                            }
                            continue;
                        }

                        shared.current_chunk.store(i, Ordering::Relaxed);
                        let _ = tts.speak(chunk, false);

                        while tts.is_speaking().unwrap_or(false) {
                            thread::sleep(std::time::Duration::from_millis(50));
                            if let Ok(cmd) = rx.try_recv() {
                                match cmd {
                                    TtsCommand::Stop => {
                                        let _ = tts.stop();
                                        shared.set_state(NarrationState::Stopped);
                                        break;
                                    }
                                    TtsCommand::SetRate(r) => {
                                        let _ = tts.set_rate(slider_to_rate(r));
                                    }
                                    TtsCommand::SetVoice(name) => {
                                        if let Some(voice) =
                                            all_voices.iter().find(|v| v.name() == name)
                                        {
                                            let _ = tts.set_voice(voice);
                                            if let Ok(mut sel) = shared.selected_voice.lock() {
                                                *sel = name;
                                            }
                                        }
                                    }
                                    TtsCommand::Quit => {
                                        let _ = tts.stop();
                                        return;
                                    }
                                    _ => {
                                        stop_flag.store(true, Ordering::Relaxed);
                                    }
                                }
                            }
                        }
                        if shared.get_state() == NarrationState::Stopped {
                            break;
                        }
                    }
                    if shared.get_state() == NarrationState::Speaking {
                        shared.set_state(NarrationState::Idle);
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
                    }
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
        let chunks = split_into_chunks(&cleaned);
        if let Ok(mut nc) = self.shared.narration_chunks.lock() {
            *nc = chunks;
        }
        let _ = self.tx.send(TtsCommand::Speak(cleaned));
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

    pub fn narration_chunks(&self) -> Vec<String> {
        self.shared.narration_chunks.lock().unwrap().clone()
    }
}

impl Drop for TtsController {
    fn drop(&mut self) {
        let _ = self.tx.send(TtsCommand::Quit);
    }
}
