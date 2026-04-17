fn main() {
    // Derive full version from git tag (supports 4-part: 0.4.3.003)
    // Falls back to CARGO_PKG_VERSION if no git tag is available.
    let cargo_version = env!("CARGO_PKG_VERSION");
    let full_version = std::process::Command::new("git")
        .args(["describe", "--tags", "--match", "v*", "--abbrev=0"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().trim_start_matches('v').to_string())
        .unwrap_or_else(|| cargo_version.to_string());

    println!("cargo:rustc-env=APP_VERSION={full_version}");
    // Rebuild if HEAD changes (new tag, new commit)
    println!("cargo:rerun-if-changed=.git/HEAD");

    #[cfg(target_os = "windows")]
    {
        // Set 8 MB default stack for all threads (including WinRT internal ones)
        println!("cargo:rustc-link-arg=/STACK:8388608");

        let mut res = winres::WindowsResource::new();
        res.set("ProductName", "ExamHelper");
        res.set("FileDescription", "ExamHelper Learning Console");
        res.set("FileVersion", &full_version);
        res.set("ProductVersion", &full_version);
        res.set_icon("assets/icon.ico");
        let _ = res.compile();
    }
}
