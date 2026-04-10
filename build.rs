fn main() {
    #[cfg(target_os = "windows")]
    {
        // Set 8 MB default stack for all threads (including WinRT internal ones)
        println!("cargo:rustc-link-arg=/STACK:8388608");

        let mut res = winres::WindowsResource::new();
        res.set("ProductName", "ExamHelper");
        res.set("FileDescription", "ExamHelper Learning Console");
        res.set_icon("assets/icon.ico");
        let _ = res.compile();
    }
}
