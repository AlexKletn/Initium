use std::io;

use drm::control::{Device, Mode, connector, crtc};
use initium_config::ShutdownConfig;

use super::card::Card;

pub(super) struct Output {
    pub connector: connector::Handle,
    pub crtc: crtc::Handle,
    pub mode: Mode,
    pub needs_modeset: bool,
}

impl Output {
    pub fn find(
        card: &Card,
        config: &ShutdownConfig,
        connector_id: Option<u32>,
    ) -> io::Result<Self> {
        let resources = card.resource_handles()?;
        let mut fallback = None;
        for &handle in resources.connectors() {
            if connector_id.is_some_and(|id| id != u32::from(handle)) {
                continue;
            }
            let output = card.get_connector(handle, false)?;
            if output.state() != connector::State::Connected {
                continue;
            }

            if let Some(encoder) = output.current_encoder()
                && let Some(crtc) = card.get_encoder(encoder)?.crtc()
                && let Some(mode) = card.get_crtc(crtc)?.mode()
            {
                return Ok(Self {
                    connector: handle,
                    crtc,
                    mode,
                    needs_modeset: false,
                });
            }

            if fallback.is_none() {
                let output = card.get_connector(handle, true)?;
                let Some(mode) = fallback_mode(output.modes(), config) else {
                    continue;
                };
                for &encoder in output.encoders() {
                    let encoder = card.get_encoder(encoder)?;
                    if let Some(&crtc) = resources.filter_crtcs(encoder.possible_crtcs()).first() {
                        fallback = Some(Self {
                            connector: handle,
                            crtc,
                            mode,
                            needs_modeset: true,
                        });
                        break;
                    }
                }
            }
        }
        fallback.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no usable shutdown output"))
    }
}

fn fallback_mode(modes: &[Mode], config: &ShutdownConfig) -> Option<Mode> {
    let matching = || {
        modes.iter().filter(|mode| {
            config
                .fallback_size
                .is_none_or(|[width, height]| mode.size() == (width, height))
        })
    };
    matching()
        .find(|mode| config.fallback_refresh_hz == Some(mode.vrefresh()))
        .or_else(|| matching().next())
        .copied()
}
