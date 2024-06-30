use std::time::Duration;

use anyhow::{bail, Context, Result};
use reqwest::Client;
use tokio::{
    process::{Child, Command},
    time::{self, Instant},
};
use tracing::{debug, info};

use crate::lm_proxy::config::ModelConfig;

pub struct LlamaCppServer {
    #[allow(unused)]
    process: Child,
    port: u16,
}

impl LlamaCppServer {
    pub async fn spawn(client: Client, config: &ModelConfig) -> Result<Self> {
        info!(config = ?config, "spawn server");

        if config.args.is_empty() {
            bail!("cannot spawn server with empty arguments");
        }

        let args = Self::prepare_args(config)?;
        let process = Command::new(&args[0])
            .args(&args[1..])
            .spawn()
            .context("Failed to start llama.cpp server")?;

        let this = Self {
            process,
            port: config.port,
        };
        this.wait_for_running(client, Instant::now() + Duration::from_secs(30))
            .await?;

        Ok(this)
    }

    pub async fn kill(&mut self) -> Result<()> {
        self.process.kill().await?;
        Ok(())
    }

    fn prepare_args(config: &ModelConfig) -> Result<Vec<String>> {
        let args = config
            .args
            .iter()
            .map(|arg| match arg.as_str() {
                "{{ port }}" => config.port.to_string(),
                arg => arg.to_owned(),
            })
            .collect::<Vec<_>>();
        Ok(args)
    }

    pub async fn wait_for_running(&self, client: Client, timeout: Instant) -> Result<()> {
        while Instant::now() < timeout {
            time::sleep(Duration::from_millis(500)).await;
            let res = match client
                .get(format!("http://localhost:{}/health", self.port))
                .send()
                .await
            {
                Ok(res) => res,
                Err(err) => {
                    debug!(err = %err, "error during request");
                    continue;
                }
            };
            if res.status() == 200 {
                return Ok(());
            }
        }
        bail!("server did not correctly start");
    }
}
