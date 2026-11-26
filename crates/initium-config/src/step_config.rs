use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct StepConfig {
    pub id: String,
    pub kind: String,
    #[serde(flatten)]
    pub parameters: toml::Table,
}
