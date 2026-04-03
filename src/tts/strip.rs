/// A chunk of text with its language code for TTS voice switching.
#[derive(Debug, Clone)]
pub struct LangChunk {
    pub text: String,
    pub lang: String,
}

/// Strip markdown syntax to produce clean text for speech.
/// `{{lang:XX}}` tags are stripped but tracked — they affect the returned LangChunks.
pub fn strip_markdown(md: &str) -> String {
    let mut out = String::with_capacity(md.len());
    for line in md.lines() {
        let trimmed = line.trim();

        // Skip horizontal rules
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            out.push('\n');
            continue;
        }

        // Preserve language switch tags as markers for chunking
        if trimmed.starts_with("{{lang:") && trimmed.ends_with("}}") {
            out.push_str(trimmed);
            out.push('\n');
            continue;
        }

        // Skip multimedia tags — don't read URLs aloud
        // Format: {{video:URL:title}} or {{image:URL:caption}}
        if trimmed.starts_with("{{video:") || trimmed.starts_with("{{image:") {
            // Extract title/caption for TTS
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

        // Skip empty lines (preserve paragraph breaks)
        if line_clean.is_empty() {
            out.push('\n');
            continue;
        }

        let result = strip_inline_markdown(line_clean);
        out.push_str(result.trim());
        out.push(' ');
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

/// Parse a `{{lang:XX}}` tag, returning the language code.
fn parse_lang_tag(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    if trimmed.starts_with("{{lang:") && trimmed.ends_with("}}") {
        Some(&trimmed[7..trimmed.len() - 2])
    } else {
        None
    }
}

/// Split text into speakable chunks annotated with language codes.
/// Language switches are triggered by `{{lang:XX}}` markers in the text.
pub fn split_into_lang_chunks(text: &str, default_lang: &str) -> Vec<LangChunk> {
    let mut chunks = Vec::new();
    let mut current_lang = default_lang.to_string();
    let mut current_text = String::new();

    for paragraph in text.split("\n\n") {
        let p = paragraph.trim();
        if p.is_empty() {
            if !current_text.trim().is_empty() {
                current_text.push_str("\n\n");
            }
            continue;
        }

        // Check each line for lang tags
        for line in p.lines() {
            if let Some(lang) = parse_lang_tag(line) {
                // Flush accumulated text as chunks in the current language
                flush_text_as_chunks(&current_text, &current_lang, &mut chunks);
                current_text.clear();
                current_lang = lang.to_string();
            } else {
                current_text.push_str(line.trim());
                current_text.push(' ');
            }
        }
        // Paragraph boundary
        if !current_text.trim().is_empty() {
            current_text.push_str("\n\n");
        }
    }

    // Flush remaining text
    flush_text_as_chunks(&current_text, &current_lang, &mut chunks);

    chunks.retain(|c| c.text.len() > 2);
    chunks
}

/// Split a block of same-language text into reasonably sized chunks.
fn flush_text_as_chunks(text: &str, lang: &str, chunks: &mut Vec<LangChunk>) {
    for paragraph in text.split("\n\n") {
        let p = paragraph.trim();
        if p.is_empty() {
            continue;
        }

        if p.len() < 300 {
            chunks.push(LangChunk {
                text: p.to_string(),
                lang: lang.to_string(),
            });
        } else {
            let mut current = String::new();
            for part in p.split(". ") {
                if current.len() + part.len() > 250 && !current.is_empty() {
                    chunks.push(LangChunk {
                        text: current.trim().to_string(),
                        lang: lang.to_string(),
                    });
                    current.clear();
                }
                current.push_str(part);
                current.push_str(". ");
            }
            if !current.trim().is_empty() {
                chunks.push(LangChunk {
                    text: current.trim().to_string(),
                    lang: lang.to_string(),
                });
            }
        }
    }
}

/// Strip `{{lang:XX}}` tags from markdown for display (not TTS).
pub fn strip_lang_tags(content: &str) -> String {
    content
        .lines()
        .filter(|line| parse_lang_tag(line).is_none())
        .collect::<Vec<_>>()
        .join("\n")
}

// Keep backward compat — old split_into_chunks is now a thin wrapper
#[allow(dead_code)]
pub fn split_into_chunks(text: &str) -> Vec<String> {
    split_into_lang_chunks(text, "en")
        .into_iter()
        .map(|c| c.text)
        .collect()
}
