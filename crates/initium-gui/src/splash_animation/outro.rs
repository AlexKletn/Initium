use std::time::{Duration, Instant};

use initium_config::SplashOutroConfig;

pub(super) struct Outro {
    config: SplashOutroConfig,
    started: Instant,
    source: Vec<u32>,
    width: usize,
    height: usize,
    satellite: (f64, f64),
}

impl Outro {
    pub fn new(
        started: Instant,
        source: &[u32],
        width: usize,
        height: usize,
        config: SplashOutroConfig,
    ) -> Self {
        let mut outro = Self {
            config,
            started,
            source: source.to_vec(),
            width,
            height,
            satellite: (0.0, 0.0),
        };
        outro.satellite = outro.satellite_center();
        outro
    }

    pub fn restore(&self, pixels: &mut [u32]) {
        pixels.copy_from_slice(&self.source);
    }

    pub fn draw(&self, now: Instant, pixels: &mut [u32]) -> bool {
        let elapsed = now.saturating_duration_since(self.started);
        let merge_time = Duration::from_millis(self.config.merge_ms.get().into());
        let collapse_time = Duration::from_millis(self.config.collapse_ms.get().into());
        let merge = fraction(elapsed, merge_time);
        let collapse = fraction(elapsed.saturating_sub(merge_time), collapse_time);
        let approach = merge * merge * (3.0 - 2.0 * merge);
        let scale = 1.0 - collapse * collapse;
        let dx = (self.satellite.0 * approach).round() as i32;
        let dy = (self.satellite.1 * approach).round() as i32;

        let radius = self.config.region_radius_px.get() as i32;
        for y in -radius..=radius {
            for x in -radius..=radius {
                let core = if scale > 0.0 {
                    self.sample(
                        (f64::from(x) / scale).round() as i32,
                        (f64::from(y) / scale).round() as i32,
                        true,
                    )
                } else {
                    0
                };
                let satellite = if merge < 1.0 {
                    fade(self.sample(x + dx, y + dy, false), 1.0 - approach.powi(4))
                } else {
                    0
                };
                pixels[self.index(x, y)] = blend(core, satellite);
            }
        }
        elapsed < merge_time + collapse_time
    }

    fn index(&self, x: i32, y: i32) -> usize {
        (self.height / 2).saturating_add_signed(y as isize) * self.width
            + (self.width / 2).saturating_add_signed(x as isize)
    }

    fn sample(&self, x: i32, y: i32, core: bool) -> u32 {
        let radius = self.config.region_radius_px.get() as i32;
        if !(-radius..=radius).contains(&x) || !(-radius..=radius).contains(&y) {
            return 0;
        }
        let core_radius = i64::from(self.config.core_radius_px.get());
        if (i64::from(x).pow(2) + i64::from(y).pow(2) <= core_radius.pow(2)) != core {
            return 0;
        }
        self.source[self.index(x, y)]
    }

    fn satellite_center(&self) -> (f64, f64) {
        let [red_weight, green_weight, blue_weight] =
            self.config.satellite_rgb_weights.map(i64::from);
        let (mut x_sum, mut y_sum, mut weight) = (0_i64, 0_i64, 0_i64);
        let radius = self.config.region_radius_px.get() as i32;
        for y in -radius..=radius {
            for x in -radius..=radius {
                let [blue, green, red, alpha] = self.sample(x, y, false).to_le_bytes();
                let pixel_weight = (i64::from(red) * red_weight
                    + i64::from(green) * green_weight
                    + i64::from(blue) * blue_weight)
                    * i64::from(alpha);
                x_sum += i64::from(x) * pixel_weight;
                y_sum += i64::from(y) * pixel_weight;
                weight += pixel_weight;
            }
        }
        let weight = weight.max(1) as f64;
        (x_sum as f64 / weight, y_sum as f64 / weight)
    }
}

fn fraction(elapsed: Duration, duration: Duration) -> f64 {
    (elapsed.as_secs_f64() / duration.as_secs_f64()).min(1.0)
}

fn fade(pixel: u32, opacity: f64) -> u32 {
    let mut channels = pixel.to_le_bytes();
    channels[3] = (f64::from(channels[3]) * opacity) as u8;
    u32::from_le_bytes(channels)
}

fn blend(core: u32, satellite: u32) -> u32 {
    let (core, satellite) = (core.to_le_bytes(), satellite.to_le_bytes());
    let alpha = u32::from(core[3].max(satellite[3]));
    if alpha == 0 {
        return 0;
    }
    let mut pixel = [0; 4];
    for channel in 0..3 {
        let core_value = u32::from(core[channel]) * u32::from(core[3]);
        let satellite_value = u32::from(satellite[channel]) * u32::from(satellite[3]);
        pixel[channel] = (core_value.max(satellite_value) / alpha) as u8;
    }
    pixel[3] = alpha as u8;
    u32::from_le_bytes(pixel)
}

#[cfg(test)]
#[path = "outro.tests.rs"]
mod tests;
