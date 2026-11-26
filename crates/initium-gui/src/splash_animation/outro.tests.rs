use std::num::NonZeroU32;

use super::*;

const MERGE: Duration = Duration::from_millis(360);
const COLLAPSE: Duration = Duration::from_millis(240);

fn config() -> SplashOutroConfig {
    SplashOutroConfig {
        merge_ms: NonZeroU32::new(360).unwrap(),
        collapse_ms: NonZeroU32::new(240).unwrap(),
        core_radius_px: NonZeroU32::new(80).unwrap(),
        region_radius_px: NonZeroU32::new(150).unwrap(),
        satellite_rgb_weights: [0, 1, 0],
    }
}

#[test]
fn merges_satellite_before_collapsing_the_core() {
    let (width, height) = (480, 540);
    let mut pixels = vec![0; width * height];
    let center = (height / 2) * width + width / 2;
    let satellite = center + 112;
    pixels[center] = 0xff80_c0ff;
    pixels[satellite] = 0xff00_c080;
    pixels[center - 125] = 0xffff_0000;
    let now = Instant::now();
    let outro = Outro::new(now, &pixels, width, height, config());

    assert_eq!(outro.satellite, (112.0, 0.0));
    assert!(outro.draw(now, &mut pixels));
    assert_ne!(pixels[satellite], 0);
    assert!(outro.draw(now + MERGE, &mut pixels));
    assert_eq!(pixels[satellite], 0);
    assert_ne!(pixels[center], 0);
    assert!(!outro.draw(now + MERGE + COLLAPSE, &mut pixels));
    assert_eq!(pixels[center], 0);
}

#[test]
fn restores_the_logo_after_the_outro_finishes() {
    let (width, height) = (480, 540);
    let mut source = vec![0; width * height];
    let center = (height / 2) * width + width / 2;
    source[center] = 0xff80_c0ff;
    source[center + 112] = 0xffff_c080;
    let mut pixels = source.clone();
    let now = Instant::now();
    let outro = Outro::new(now, &source, width, height, config());

    assert!(!outro.draw(now + MERGE + COLLAPSE, &mut pixels));
    outro.restore(&mut pixels);
    assert_eq!(pixels, source);
}
