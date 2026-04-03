/// A contiguous range of same-language text, pre-split into speakable chunks.
/// The TTS worker exhausts all chunks in one range before switching voice.
#[derive(Debug, Clone)]
pub struct LangRange {
    pub lang: String,
    pub chunks: Vec<String>,
}

/// Strip markdown syntax to produce clean text for speech.
/// Language tags (`{{lang:XX}}`, `{{/lang}}`) are preserved as markers for range splitting.
pub fn strip_markdown(md: &str) -> String {
    let mut out = String::with_capacity(md.len());
    for line in md.lines() {
        let trimmed = line.trim();

        // Skip horizontal rules
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            out.push('\n');
            continue;
        }

        // Preserve language tags as markers
        if parse_lang_open(trimmed).is_some() || is_lang_close(trimmed) {
            out.push_str(trimmed);
            out.push('\n');
            continue;
        }

        // Skip multimedia tags — only read title/caption
        if trimmed.starts_with("{{video:") || trimmed.starts_with("{{image:") {
            if let Some(last_colon) = trimmed.rfind(':') {
                let title = &trimmed[last_colon + 1..trimmed.len().saturating_sub(2)];
                let title = title.trim();
                if !title.is_empty() {
                    out.push_str(title);
                    out.push_str(". ");
                }
            }
            continue;
        }

        // Handle table rows: split cells, skip separator rows
        if trimmed.starts_with('|') && trimmed.ends_with('|') {
            let inner = &trimmed[1..trimmed.len() - 1];
            let is_separator = inner.split('|').all(|cell| {
                let c = cell.trim();
                !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':')
            });
            if is_separator {
                continue;
            }
            let cells: Vec<&str> = inner
                .split('|')
                .map(|c| c.trim())
                .filter(|c| !c.is_empty())
                .collect();
            for (i, cell) in cells.iter().enumerate() {
                let clean = strip_inline_markdown(cell);
                out.push_str(&clean);
                if i + 1 < cells.len() {
                    out.push_str(", ");
                }
            }
            out.push_str(".\n");
            continue;
        }

        // Strip heading markers
        let line_clean = if trimmed.starts_with('#') {
            trimmed.trim_start_matches('#').trim()
        } else {
            trimmed
        };

        if line_clean.is_empty() {
            out.push('\n');
            continue;
        }

        let result = strip_inline_markdown(line_clean);
        out.push_str(result.trim());
        out.push('\n');
    }
    out
}

/// Strip inline markdown formatting (bold, italic, code, links).
fn strip_inline_markdown(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' | '_' => {
                while chars.peek() == Some(&'*') || chars.peek() == Some(&'_') {
                    chars.next();
                }
            }
            '`' => {
                while chars.peek() == Some(&'`') {
                    chars.next();
                }
            }
            '[' => {
                let mut link_text = String::new();
                for lc in chars.by_ref() {
                    if lc == ']' {
                        break;
                    }
                    link_text.push(lc);
                }
                if chars.peek() == Some(&'(') {
                    chars.next();
                    for lc in chars.by_ref() {
                        if lc == ')' {
                            break;
                        }
                    }
                }
                result.push_str(&link_text);
            }
            '|' => {
                result.push_str(", ");
            }
            '-' if text.starts_with("- ") && result.is_empty() => {
                result.push(' ');
            }
            _ => result.push(c),
        }
    }
    result
}

/// Parse `{{lang:XX}}` — returns the language code.
fn parse_lang_open(line: &str) -> Option<&str> {
    let t = line.trim();
    if t.starts_with("{{lang:") && t.ends_with("}}") && !t.contains("/lang") {
        Some(&t[7..t.len() - 2])
    } else {
        None
    }
}

/// Check if line is `{{/lang}}`.
fn is_lang_close(line: &str) -> bool {
    line.trim() == "{{/lang}}"
}

/// Split stripped text into language ranges. Each range is a contiguous block
/// of same-language text, internally split into speakable chunks.
pub fn split_into_ranges(text: &str, default_lang: &str) -> Vec<LangRange> {
    let mut ranges: Vec<LangRange> = Vec::new();
    let mut current_lang = default_lang.to_string();
    let mut current_text = String::new();

    for line in text.lines() {
        if let Some(lang) = parse_lang_open(line) {
            // Flush accumulated text as a range
            flush_range(&current_text, &current_lang, &mut ranges);
            current_text.clear();
            current_lang = lang.to_string();
        } else if is_lang_close(line) {
            // Flush and return to default
            flush_range(&current_text, &current_lang, &mut ranges);
            current_text.clear();
            current_lang = default_lang.to_string();
        } else {
            current_text.push_str(line);
            current_text.push('\n');
        }
    }

    // Flush remaining
    flush_range(&current_text, &current_lang, &mut ranges);
    ranges
}

/// Split a block of text into speakable chunks and push as a LangRange.
fn flush_range(text: &str, lang: &str, ranges: &mut Vec<LangRange>) {
    let mut chunks = Vec::new();
    for paragraph in text.split("\n\n") {
        let p = paragraph.trim();
        if p.is_empty() {
            continue;
        }
        if p.len() < 300 {
            chunks.push(p.to_string());
        } else {
            let mut current = String::new();
            for part in p.split(". ") {
                if current.len() + part.len() > 250 && !current.is_empty() {
                    chunks.push(current.trim().to_string());
                    current.clear();
                }
                current.push_str(part);
                current.push_str(". ");
            }
            if !current.trim().is_empty() {
                chunks.push(current.trim().to_string());
            }
        }
    }
    chunks.retain(|c| c.len() > 2);
    if !chunks.is_empty() {
        ranges.push(LangRange {
            lang: lang.to_string(),
            chunks,
        });
    }
}
/// Flatten ranges into (text, lang) pairs for karaoke display.
pub fn flatten_ranges(ranges: &[LangRange]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for range in ranges {
        for chunk in &range.chunks {
            out.push((chunk.clone(), range.lang.clone()));
        }
    }
    out
}
