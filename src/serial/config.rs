use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SerialConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uart: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baud: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net_bind: Option<String>,
}
