fn main() {
    const ICON_PATH: &str = "assets/branding/Insta360Linker.ico";

    println!("cargo:rerun-if-changed={ICON_PATH}");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon(ICON_PATH)
        .set("ProductName", "Insta360Linker")
        .set("FileDescription", "Insta360Linker")
        .set("InternalName", "Insta360Linker")
        .set("OriginalFilename", "Insta360Linker.exe");
    resource
        .compile()
        .expect("failed to compile the Insta360Linker Windows resources");
}
