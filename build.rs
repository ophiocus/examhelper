fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set("ProductName", "ExamHelper");
        res.set("FileDescription", "ExamHelper Learning Console");
        res.set_icon("assets/icon.ico");
        let _ = res.compile();
    }
}
