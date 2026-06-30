fn main() {
    // Rebuild when tags change so the embedded Windows resource picks up
    // version bumps that happen as part of a tag-push.
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/tags");

    #[cfg(target_os = "windows")]
    {
        // 8 MB default stack for all threads (including WinRT internal ones).
        // Smaller defaults overflow in the speech-synthesis path.
        println!("cargo:rustc-link-arg=/STACK:8388608");

        let version = env!("CARGO_PKG_VERSION");
        let mut res = winres::WindowsResource::new();
        res.set("ProductName", env!("CARGO_PKG_NAME"));
        res.set("FileDescription", env!("CARGO_PKG_DESCRIPTION"));
        res.set("FileVersion", version);
        res.set("ProductVersion", version);
        res.set_icon("assets/icon.ico");
        let _ = res.compile();
    }
}
