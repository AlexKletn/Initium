use std::io;
use std::os::fd::{AsFd, AsRawFd};

use drm::Device as _;
use drm::buffer::{Buffer, DrmFourcc};
use drm::control::{Device as _, Event, PageFlipFlags, crtc, framebuffer};
use initium_config::ShutdownConfig;

use super::{caption::Caption, card::Card, output::Output};

unsafe extern "C" {
    fn drmModeCloseFB(fd: libc::c_int, framebuffer: u32) -> libc::c_int;
}

pub(super) struct Frame {
    card: Card,
    framebuffer: framebuffer::Handle,
}

impl Frame {
    pub fn present(
        config: &ShutdownConfig,
        connector_id: Option<u32>,
        caption: &Caption,
    ) -> io::Result<Self> {
        let card = Card::open(&config.drm_device)?;
        let output = Output::find(&card, config, connector_id)?;
        let (width, height) = output.mode.size();
        let mut buffer = card.create_dumb_buffer(
            (u32::from(width), u32::from(height)),
            DrmFourcc::Xrgb8888,
            32,
        )?;
        let stride = buffer.pitch();
        let framebuffer = card.add_framebuffer(&buffer, 24, 32)?;
        caption.render_into(
            card.map_dumb_buffer(&mut buffer)?.as_mut(),
            (width, height),
            stride,
        )?;

        card.acquire_master_lock()?;
        let presented = if output.needs_modeset {
            card.set_crtc(
                output.crtc,
                Some(framebuffer),
                (0, 0),
                &[output.connector],
                Some(output.mode),
            )
        } else {
            card.page_flip(output.crtc, framebuffer, PageFlipFlags::EVENT, None)
                .and_then(|()| wait_for_flip(&card, output.crtc))
        };
        let released = card.release_master_lock();
        presented.and(released)?;
        Ok(Self { card, framebuffer })
    }

    pub fn leave_on_screen(self) -> io::Result<()> {
        // SAFETY: The live card owns this framebuffer. CLOSEFB preserves the scanout reference.
        let result =
            unsafe { drmModeCloseFB(self.card.as_fd().as_raw_fd(), self.framebuffer.into()) };
        if result < 0 {
            return Err(io::Error::from_raw_os_error(-result));
        }
        Ok(())
    }
}

fn wait_for_flip(card: &Card, crtc: crtc::Handle) -> io::Result<()> {
    let mut poll_fd = libc::pollfd {
        fd: card.as_fd().as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: The sole poll descriptor is initialized and the card remains open.
    let ready = unsafe { libc::poll(&mut poll_fd, 1, 1500) };
    if ready < 0 {
        return Err(io::Error::last_os_error());
    }
    if ready > 0
        && poll_fd.revents & libc::POLLIN != 0
        && card
            .receive_events()?
            .any(|event| matches!(event, Event::PageFlip(flip) if flip.crtc == crtc))
    {
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        "shutdown page flip was not confirmed",
    ))
}
