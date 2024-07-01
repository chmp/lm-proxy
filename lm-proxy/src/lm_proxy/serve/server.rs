use std::{collections::HashSet, sync::Arc, time::Duration};

use anyhow::Result;
use axum::{
    body::Body,
    extract::{Path, State},
    http::{Request, Response, Uri},
    response::IntoResponse,
    routing::{any, get},
    Router,
};
use serde::Deserialize;
use tokio::{
    sync::mpsc::Receiver,
    time::{self, Instant},
};
use tower_http::trace::TraceLayer;
use tracing::{error, info};

use super::server_state::{ModelServer, ServerState};
use crate::lm_proxy::{error::ServerResult, utils};

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModelName(String);

pub fn build_router(state: Arc<ServerState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/proxy/:model/*path", any(proxy_request))
        .route("/v1/*path", any(proxy_with_model_in_request))
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone())
}

pub async fn cleanup_models(state: Arc<ServerState>, mut stop: Receiver<()>) {
    loop {
        tokio::select! {
            biased;
            _ = stop.recv() => { break; }
            _ = time::sleep(Duration::from_secs(10)) => {}
        }

        time::sleep(Duration::from_secs(10)).await;
        let mut to_kill = Vec::new();

        {
            let mut servers = state.servers.lock().await;
            let now = Instant::now();

            let mut evict = HashSet::<String>::new();

            for (model, info) in servers.iter() {
                if !info.is_alive(now) {
                    evict.insert(model.clone());
                    to_kill.push(info.clone());
                }
            }
            if !evict.is_empty() {
                info!(models = ?evict, "evict models");
                servers.retain(|model, _| !evict.contains(model));
            }
        }

        for info in to_kill {
            info!(name = %info.name, "kill model server");
            let mut server = info.server.lock().await;
            let res = server.kill().await;
            if let Err(err) = res {
                error!(err = %err, "error while killing model server");
            }
        }
    }
}

async fn health() -> String {
    String::from("ok")
}

async fn proxy_request(
    Path((model, path)): Path<(String, String)>,
    State(state): State<Arc<ServerState>>,
    mut req: Request<Body>,
) -> ServerResult<Response<Body>> {
    info!(model, path, "proxy");

    let server_info = state.ensure_model_with_default_keep_alive(&model).await?;

    let uri = format!("http://localhost:{}/{}", server_info.port, path);
    *req.uri_mut() = uri.parse::<Uri>()?;

    let res = utils::forward_request(&state.client, req)
        .await
        .into_response();
    let res = keep_model_alive(res, server_info);

    Ok(res)
}

async fn proxy_with_model_in_request(
    Path(path): Path<String>,
    State(state): State<Arc<ServerState>>,
    req: Request<Body>,
) -> ServerResult<Response<Body>> {
    info!(path, "proxy");
    let (mut req, model) = peek_model(req, 512 * 1024).await?;

    let server_info = state.ensure_model_with_default_keep_alive(&model).await?;

    let uri = format!("http://localhost:{}/v1/{}", server_info.port, path);
    *req.uri_mut() = uri.parse::<Uri>()?;

    let res = utils::forward_request(&state.client, req)
        .await
        .into_response();
    let res = keep_model_alive(res, server_info);

    Ok(res)
}

/// Determine the model from the request body
async fn peek_model(mut req: Request<Body>, max_bytes: usize) -> Result<(Request<Body>, String)> {
    let body = std::mem::take(req.body_mut());
    let body_bytes = axum::body::to_bytes(body, max_bytes).await?;

    #[derive(Debug, Deserialize)]
    struct PartialRequest {
        #[allow(unused)]
        model: String,
    }

    let PartialRequest { model } = serde_json::from_slice(&body_bytes)?;
    let _ = std::mem::replace(req.body_mut(), Body::from(body_bytes));

    Ok((req, model))
}

/// Keep the model alive until the response body is consumed
fn keep_model_alive(mut res: Response<Body>, model_server: Arc<ModelServer>) -> Response<Body> {
    let token = model_server.new_keep_alive_token_with_default();
    utils::add_cleanup_to_body(res.body_mut(), move || {
        info!(model = %model_server.name, "invalidate keep alive token");
        model_server.invalidate_keep_alive_token(token);
    });

    res
}
