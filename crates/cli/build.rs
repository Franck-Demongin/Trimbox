fn main() {
    #[cfg(all(feature = "gui", target_os = "windows"))]
    {
        let mut res = winresource::WindowsResource::new();
        let icon =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/icons/trimbox.ico");
        res.set_icon(icon.to_string_lossy().as_ref());
        res.set("FileDescription", "Préparation et validation PDF/X-4");
        res.set("ProductName", "Trimbox");
        res.set("OriginalFilename", "trimbox.exe");
        res.compile()
            .expect("échec de la compilation des ressources Windows");
    }

    #[cfg(feature = "gui")]
    {
        if let Err(error) = slint_build::compile("ui/main_window.slint") {
            panic!("compilation Slint impossible : {error}");
        }
    }
}
