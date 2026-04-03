/// Strip markdown syntax to produce clean text for speech.
pub fn strip_markdown(md: &str) -> String {
    let mut out = String::with_capacity(md.len());
    for line in md.lines() {
        let trimmed = line.trim();

        // Skip horizontal rules
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
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

/// Split text into speakable chunks (by paragraph, then by sentence).
pub fn split_into_chunks(text: &str) -> Vec<String> {
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
    chunks
}
