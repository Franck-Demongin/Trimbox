fn main() {
    #[cfg(feature = "gui")]
    {
        if let Err(error) = slint_build::compile("ui/main_window.slint") {
            panic!("compilation Slint impossible : {error}");
        }
    }
}
