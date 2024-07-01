pub mod server;
pub mod server_state;

use std::{net::Ipv4Addr, sync::Arc};

use anyhow::Result;
use server::build_router;
use server_state::ServerState;
use tokio::net::TcpListener;
use tracing::info;

use crate::lm_proxy::config::Config;

use super::cmd::ServeCommand;

impl ServeCommand {
    pub fn run(self) -> Result<()> {
        tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(self.run_impl())
    }

    async fn run_impl(self) -> Result<()> {
        let config = Config::form_path(&self.config)?;
        info!(config = ?config, "loaded config");

        let server_port = config.proxy.port;
        let state = Arc::new(ServerState::from_config(config));
        let router = build_router(state.clone());

        tracing_subscriber::fmt::init();

        let task = tokio::spawn(server::cleanup_models(state));

        let addr = (Ipv4Addr::new(127, 0, 0, 1), server_port);
        let listener = TcpListener::bind(addr).await?;

        axum::serve(listener, router).await?;
        task.await?;

        Ok(())
    } 
}