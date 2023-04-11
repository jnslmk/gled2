fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winresource::WindowsResource::new();
        res.set_ar_path("x86_64-w64-mingw32-gcc-ar");
        res.set_windres_path("x86_64-w64-mingw32-windres");
        res.set_icon("assets/logo.ico");
        res.compile().expect("Could not compile");
    }
}
