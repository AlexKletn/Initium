fn main() {
    let source = "src/bootloader_frame/framebuffer.c";
    println!("cargo:rerun-if-changed={source}");
    let drm = "src/display/drm_scanout.c";
    println!("cargo:rerun-if-changed={drm}");
    cc::Build::new()
        .files([source, drm])
        .compile("initium_display");
}
