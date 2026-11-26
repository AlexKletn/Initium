use serde::Deserialize;

use crate::StepConfig;
use crate::ui::UiConfig;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub shutdown: Option<crate::ShutdownConfig>,
    pub ui: UiConfig,
    pub steps: Vec<StepConfig>,
}

impl Config {
    pub fn parse(document: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(document)
    }
}

#[cfg(test)]
#[path = "config.tests.rs"]
mod tests;
