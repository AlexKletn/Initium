use initium_config::Config;

use super::*;

#[test]
fn resolves_reserved_region_and_rejects_frames_outside_it() {
    let config = Config::parse(
        r#"
        [ui.splash.bootloader_frame.reserved_memory]
        node = "memory@88000000"
        width = 1920
        height = 1200
        "#,
    )
    .expect("parse region");
    let mut region = config
        .ui
        .splash
        .bootloader_frame
        .and_then(|frame| frame.reserved_memory)
        .expect("configured region");

    let root = std::env::temp_dir().join(format!("initium-dt-{}", std::process::id()));
    let node = root.join(&region.node);
    fs::create_dir_all(&node).expect("create DT fixture");
    fs::write(root.join("#address-cells"), [0, 0, 0, 2]).expect("write address cells");
    fs::write(root.join("#size-cells"), [0, 0, 0, 2]).expect("write size cells");
    fs::write(root.join("ranges"), []).expect("write ranges");
    fs::write(node.join("no-map"), []).expect("write no-map flag");
    fs::write(
        node.join("reg"),
        [0, 0, 0, 0, 0x88, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0],
    )
    .expect("write region");

    let (offset, layout) = resolve(&root, &region).expect("resolve test framebuffer");
    assert_eq!(offset, 0x8800_0000);
    assert_eq!(
        (layout.width, layout.height, layout.stride),
        (1920, 1200, 7680)
    );

    region.height = std::num::NonZeroU32::new(4000).expect("nonzero height");
    assert!(resolve(&root, &region).is_err());
    region.node = "../memory@88000000".into();
    assert!(resolve(&root, &region).is_err());
    fs::remove_dir_all(root).expect("remove DT fixture");
}
