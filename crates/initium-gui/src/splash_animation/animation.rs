use std::fs::File;
use std::io::{self, BufReader, Seek};
use std::time::{Duration, Instant};

use initium_config::{SplashConfig, SplashOutroConfig};

use super::{frames::read_frame, image::AnimationImage, outro::Outro};
use crate::DisplayConnection;

pub struct SplashAnimation<'display> {
    image: AnimationImage<'display>,
    intro: Option<BufReader<File>>,
    repeating: Option<BufReader<File>>,
    intro_interval: Duration,
    loop_interval: Duration,
    outro_config: Option<SplashOutroConfig>,
    outro: Option<Outro>,
}

impl<'display> SplashAnimation<'display> {
    pub fn open(
        display: &'display DisplayConnection,
        splash: &SplashConfig,
    ) -> io::Result<Option<Self>> {
        if splash.intro_frames.is_none() && splash.loop_frames.is_none() {
            return Ok(None);
        }

        let config = splash.animation.as_ref().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "missing [ui.splash.animation]")
        })?;
        if config
            .outro
            .is_some_and(|outro| !outro.fits(config.width.get(), config.height.get()))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid splash outro geometry or RGB weights",
            ));
        }
        let intro = splash
            .intro_frames
            .as_ref()
            .map(File::open)
            .transpose()?
            .map(BufReader::new);
        let repeating = splash
            .loop_frames
            .as_ref()
            .map(File::open)
            .transpose()?
            .map(BufReader::new);
        let intro_interval = interval(config.intro_fps.get())?;
        let loop_interval = interval(config.loop_fps.get())?;
        let image = AnimationImage::create(display, config.width.get(), config.height.get())?;
        Ok(Some(Self {
            image,
            intro,
            repeating,
            intro_interval,
            loop_interval,
            outro_config: config.outro,
            outro: None,
        }))
    }

    pub fn set_offset(&mut self, offset: i32) -> Result<(), crate::Error> {
        self.image.set_offset(offset)
    }

    pub fn begin_outro(&mut self, now: Instant) -> bool {
        let Some(config) = self.outro_config else {
            return false;
        };
        let (pixels, width, height) = self.image.frame();
        self.outro = Some(Outro::new(now, pixels, width, height, config));
        true
    }

    pub fn cancel_outro(&mut self) -> io::Result<()> {
        if let Some(outro) = self.outro.take() {
            self.image.update(|pixels| {
                outro.restore(pixels);
                Ok(true)
            })?;
        }
        Ok(())
    }

    pub fn advance_outro(&mut self, now: Instant) -> io::Result<(Duration, bool)> {
        let outro = self
            .outro
            .as_ref()
            .ok_or_else(|| io::Error::other("splash outro has not started"))?;
        let mut ongoing = true;
        self.image.update(|pixels| {
            ongoing = outro.draw(now, pixels);
            Ok(true)
        })?;
        Ok((self.loop_interval, ongoing))
    }

    pub fn advance(&mut self) -> io::Result<Option<Duration>> {
        if let Some(intro) = &mut self.intro {
            if self.image.update(|pixels| read_frame(intro, pixels))? {
                return Ok(Some(self.intro_interval));
            }
            self.intro = None;
        }

        let Some(repeating) = &mut self.repeating else {
            return Ok(None);
        };
        let advanced = self.image.update(|pixels| {
            if read_frame(repeating, pixels)? {
                return Ok(true);
            }
            repeating.rewind()?;
            read_frame(repeating, pixels)
        })?;

        Ok(advanced.then_some(self.loop_interval))
    }
}

fn interval(fps: u32) -> io::Result<Duration> {
    if fps > 240 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "splash animation exceeds 240 FPS",
        ));
    }
    Ok(Duration::from_nanos(1_000_000_000 / u64::from(fps)))
}
