use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::Duration,
};

use anyhow::{bail, Result};
use reqwest::Client;
use tokio::{sync::Mutex, time::Instant};
use tracing::debug;

use super::config::Config;
use super::servers::llama_cpp_server::LlamaCppServer;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct KeepAliveToken(pub usize);

impl std::fmt::Display for KeepAliveToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "KeepAliveToken({})", self.0)
    }
}

pub struct ServerState {
    pub client: Client,
    pub config: Config,
    pub servers: Mutex<HashMap<String, Arc<ModelServer>>>,
}

impl ServerState {
    pub fn from_config(config: Config) -> Self {
        Self {
            client: Client::new(),
            servers: Default::default(),
            config,
        }
    }

    pub async fn ensure_model_with_default_keep_alive(
        &self,
        name: &str,
    ) -> Result<Arc<ModelServer>> {
        let keep_alive = Instant::now() + self.config.proxy.keep_alive;
        self.ensure_model(name, keep_alive).await
    }

    pub async fn ensure_model(&self, name: &str, alive_until: Instant) -> Result<Arc<ModelServer>> {
        let mut servers = self.servers.lock().await;
        if let Some(server_info) = servers.get(name) {
            server_info.update_alive_until(alive_until);
            Ok(server_info.clone())
        } else {
            let Some(model_config) = self.config.models.get(name) else {
                bail!("could not find model {name}");
            };

            let server = LlamaCppServer::spawn(self.client.clone(), model_config).await?;

            let server_info = KeepAliveState {
                alive_until,
                keep_alive_tokens: Default::default(),
                next_keep_alive_token: KeepAliveToken(0),
            };
            let server_info = Arc::new(ModelServer {
                name: name.to_owned(),
                port: model_config.port,
                keep_alive: self.config.proxy.keep_alive,
                request_keep_alive: self.config.proxy.request_keep_alive,
                state: std::sync::Mutex::new(server_info),
                server: Mutex::new(server),
            });
            servers.insert(name.to_owned(), server_info.clone());
            Ok(server_info)
        }
    }
}

pub struct ModelServer {
    pub name: String,
    pub port: u16,
    pub keep_alive: Duration,
    pub request_keep_alive: Duration,
    pub server: Mutex<LlamaCppServer>,
    state: std::sync::Mutex<KeepAliveState>,
}

pub struct KeepAliveState {
    pub alive_until: Instant,
    pub keep_alive_tokens: BTreeMap<KeepAliveToken, Instant>,
    pub next_keep_alive_token: KeepAliveToken,
}

impl ModelServer {
    pub fn is_alive(&self, now: Instant) -> bool {
        let state = self.state.lock().unwrap();
        if state.alive_until > now {
            return true;
        }
        for token_alive_until in state.keep_alive_tokens.values() {
            if *token_alive_until > now {
                return true;
            }
        }
        false
    }

    pub fn update_alive_until(&self, alive_until: Instant) {
        let mut state = self.state.lock().unwrap();
        state.alive_until = std::cmp::max(state.alive_until, alive_until);
    }

    pub fn new_keep_alive_token_with_default(&self) -> KeepAliveToken {
        let keep_alive = Instant::now() + self.request_keep_alive;
        self.new_keep_alive_token(keep_alive)
    }

    pub fn new_keep_alive_token(&self, alive_until: Instant) -> KeepAliveToken {
        let mut state = self.state.lock().unwrap();
        let res = state.next_keep_alive_token.clone();

        state.keep_alive_tokens.insert(res.clone(), alive_until);
        state.next_keep_alive_token = KeepAliveToken(state.next_keep_alive_token.0 + 1);

        res
    }

    pub fn invalidate_keep_alive_token(&self, token: KeepAliveToken) {
        let mut state = self.state.lock().unwrap();

        state.alive_until = std::cmp::max(state.alive_until, Instant::now() + self.keep_alive);

        let present = state.keep_alive_tokens.remove(&token).is_some();
        if !present {
            debug!(token = %token, "token already invalidated");
        }
    }
}
