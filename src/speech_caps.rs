use std::process::Command;

#[derive(Debug, Clone)]
pub struct SpeechCapability {
    pub name: String,
    pub installed: bool,
}

/// Query Windows for available speech language packs.
pub fn query_speech_capabilities() -> Vec<SpeechCapability> {
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            r#"Get-WindowsCapability -Online -Name 'Language.Speech*' | ForEach-Object { "$($_.Name)|$($_.State)" }"#,
        ])
        .output();

    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|line| {
                let parts: Vec<&str> = line.splitn(2, '|').collect();
                let name = parts.first().unwrap_or(&"").trim().to_string();
                let state = parts.get(1).unwrap_or(&"").trim().to_string();
                SpeechCapability {
                    name,
                    installed: state == "Installed",
                }
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Install a speech capability (requires elevation).
pub fn install_speech_capability(cap_name: &str) {
    let script = format!("Add-WindowsCapability -Online -Name '{}'", cap_name);
    let _ = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!(
                "Start-Process powershell -Verb RunAs -ArgumentList '-NoProfile','-Command','{}' -Wait",
                script.replace('\'', "''")
            ),
        ])
        .spawn();
}
