use std::{net::Ipv4Addr, sync::Arc};

use anyhow::Result;
use lm_proxy::{
    config::{Args, Config},
    server::{self, build_router},
    server_state::ServerState,
};
use tokio::net::TcpListener;
use tracing::info;

mod lm_proxy;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Args = argh::from_env();
    let config = Config::form_args(&args)?;
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
