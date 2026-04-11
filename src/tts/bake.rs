use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::mpsc;

use windows::Media::SpeechSynthesis::{SpeechSynthesizer, VoiceInformation};
use windows::Storage::Streams::{DataReader, InputStreamOptions};

use super::cache::{hash_voice_config, TtsCache};
use super::LangRange;

/// Progress update sent from the bake thread.
#[derive(Debug, Clone)]
pub struct BakeProgress {
    pub current: usize,
    pub total: usize,
    /// Number of chunks that were already cached (not rebaked).
    pub skipped: usize,
    pub done: bool,
    pub error: Option<String>,
}

/// Synthesize a single text chunk to a WAV file using WinRT directly.
fn synthesize_to_wav(
    synth: &SpeechSynthesizer,
    text: &str,
    output: &Path,
) -> Result<(), String> {
    let hstring: windows::core::HSTRING = text.into();
    let stream = synth
        .SynthesizeTextToStreamAsync(&hstring)
        .map_err(|e| format!("SynthesizeTextToStreamAsync: {e}"))?
        .get()
        .map_err(|e| format!("Await stream: {e}"))?;

    let size = stream.Size().map_err(|e| format!("Stream size: {e}"))? as u32;

    let input_stream = stream
        .GetInputStreamAt(0)
        .map_err(|e| format!("GetInputStreamAt: {e}"))?;
    let reader =
        DataReader::CreateDataReader(&input_stream).map_err(|e| format!("DataReader: {e}"))?;
    reader
        .SetInputStreamOptions(InputStreamOptions::ReadAhead)
        .map_err(|e| format!("SetInputStreamOptions: {e}"))?;
    reader
        .LoadAsync(size)
        .map_err(|e| format!("LoadAsync: {e}"))?
        .get()
        .map_err(|e| format!("Await load: {e}"))?;

    let mut buf = vec![0u8; size as usize];
    reader
        .ReadBytes(&mut buf)
        .map_err(|e| format!("ReadBytes: {e}"))?;

    if let Some(parent) = output.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(output, &buf).map_err(|e| format!("Write WAV: {e}"))?;

    Ok(())
}

/// Find a WinRT voice matching a language code.
fn find_voice_for_lang(
    lang: &str,
    voice_map: &HashMap<String, String>,
) -> Result<VoiceInformation, String> {
    let all_voices = SpeechSynthesizer::AllVoices()
        .map_err(|e| format!("AllVoices: {e}"))?;

    // Check voice map first
    if let Some(voice_name) = voice_map.get(lang) {
        for i in 0..all_voices.Size().unwrap_or(0) {
            if let Ok(v) = all_voices.GetAt(i) {
                if let Ok(name) = v.DisplayName() {
                    if name.to_string() == *voice_name {
                        return Ok(v);
                    }
                }
            }
        }
    }

    // Fallback: prefix match on language
    let ll = lang.to_lowercase();
    for i in 0..all_voices.Size().unwrap_or(0) {
        if let Ok(v) = all_voices.GetAt(i) {
            if let Ok(vlang) = v.Language() {
                if vlang.to_string().to_lowercase().starts_with(&ll) {
                    return Ok(v);
                }
            }
        }
    }

    Err(format!("No voice found for language: {lang}"))
}

/// Bake all chunks for a content file. Only rebakes chunks whose text changed.
/// Sends progress updates through the channel.
pub fn bake_content(
    ranges: &[LangRange],
    voice_map: &HashMap<String, String>,
    rate: f32,
    cache: &TtsCache,
    cartridge_id: &str,
    file_stem: &str,
    progress_tx: &mpsc::Sender<BakeProgress>,
) -> Result<usize, String> {
    let voice_config_hash = hash_voice_config(voice_map, rate);
    let check = cache.check(cartridge_id, file_stem, ranges, &voice_config_hash);

    let total = check.total;
    let missing = check.missing_indices();
    let skipped = check.cached;

    if missing.is_empty() {
        let _ = progress_tx.send(BakeProgress {
            current: total,
            total,
            skipped,
            done: true,
            error: None,
        });
        return Ok(skipped);
    }

    // Create synthesizer
    let synth = SpeechSynthesizer::new().map_err(|e| format!("SpeechSynthesizer::new: {e}"))?;
    let options = synth.Options().map_err(|e| format!("Options: {e}"))?;
    options
        .SetSpeakingRate(rate as f64)
        .map_err(|e| format!("SetSpeakingRate: {e}"))?;

    // Flatten ranges for indexed access
    let flat: Vec<(&str, &str)> = ranges
        .iter()
        .flat_map(|r| r.chunks.iter().map(move |c| (c.as_str(), r.lang.as_str())))
        .collect();

    let mut current_voice_lang = String::new();
    let mut baked_count = 0usize;

    for &idx in &missing {
        let (text, lang) = flat[idx];

        // Switch voice if language changed
        if lang != current_voice_lang {
            match find_voice_for_lang(lang, voice_map) {
                Ok(voice) => {
                    let _ = synth.SetVoice(&voice);
                    current_voice_lang = lang.to_string();
                }
                Err(e) => {
                    let _ = progress_tx.send(BakeProgress {
                        current: baked_count + skipped,
                        total,
                        skipped,
                        done: true,
                        error: Some(e.clone()),
                    });
                    return Err(e);
                }
            }
        }

        let wav_path = cache.wav_path(cartridge_id, file_stem, idx, lang);
        if let Err(e) = synthesize_to_wav(&synth, text, &wav_path) {
            let _ = progress_tx.send(BakeProgress {
                current: baked_count + skipped,
                total,
                skipped,
                done: true,
                error: Some(e.clone()),
            });
            return Err(e);
        }

        baked_count += 1;
        let _ = progress_tx.send(BakeProgress {
            current: baked_count + skipped,
            total,
            skipped,
            done: false,
            error: None,
        });
    }

    // Save updated manifest
    let manifest = TtsCache::build_manifest(ranges, &voice_config_hash);
    cache.save_manifest(cartridge_id, file_stem, &manifest);

    let _ = progress_tx.send(BakeProgress {
        current: total,
        total,
        skipped,
        done: true,
        error: None,
    });

    Ok(skipped)
}
