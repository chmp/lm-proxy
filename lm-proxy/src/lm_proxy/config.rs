use std::{collections::HashMap, fs, path::PathBuf, time::Duration};

use anyhow::Result;
use argh::FromArgs;
use serde::{Deserialize, Serialize};

#[derive(FromArgs)]
/// llm_proxy - a proxy for LLM models
pub struct Args {
    /// the toml config file to parse
    #[argh(positional)]
    pub config: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub proxy: ProxyConfig,
    pub models: HashMap<String, ModelConfig>,
}

impl Config {
    pub fn form_args(args: &Args) -> Result<Self> {
        let config_content = fs::read_to_string(&args.config)?;
        let mut config: Config = toml::from_str(&config_content)?;

        let mut used_auto_ports = 0;
        for model in config.models.values_mut() {
            if model.port == 0 {
                model.port = config.proxy.port + 1 + used_auto_ports;
                used_auto_ports += 1;
            }
        }

        Ok(config)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub port: u16,
    #[serde(with = "duration_as_secs")]
    pub keep_alive: Duration,
    #[serde(with = "duration_as_secs")]
    pub request_keep_alive: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelConfig {
    pub args: Vec<String>,
    #[serde(default)]
    pub port: u16,
}

mod duration_as_secs {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(
        duration: &Duration,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Duration, D::Error> {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}
