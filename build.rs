fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set("ProductName", "ExamHelper");
        res.set("FileDescription", "Colombian Naturalization Exam Study Helper");
        // res.set_icon("assets/icon.ico"); // uncomment when icon is available
        let _ = res.compile();
    }
}
