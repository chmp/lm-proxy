use anyhow::Result;
use axum::{body::Body, extract::Request, response::Response};
use reqwest::Client;

use crate::lm_proxy::error::ServerResult;

pub async fn forward_request(client: &Client, req: Request<Body>) -> ServerResult<Response<Body>> {
    let req = axum_to_reqwest(client, req)?;
    let res = client.execute(req).await?;
    Ok(reqwest_to_axum(res)?)
}

fn axum_to_reqwest(client: &Client, mut req: Request<Body>) -> Result<reqwest::Request> {
    let method = std::mem::take(req.method_mut());
    let uri = std::mem::take(req.uri_mut());
    let headers = std::mem::take(req.headers_mut());

    let req_body = sync_wrapper::SyncStream::new(req.into_body().into_data_stream());
    let req_body = reqwest::Body::wrap_stream(req_body);

    let req = client
        .request(method, uri.to_string())
        .body(req_body)
        .headers(headers)
        .build()?;
    Ok(req)
}

fn reqwest_to_axum(res: reqwest::Response) -> Result<Response<Body>> {
    let mut response_builder = Response::builder().status(res.status().as_u16());
    for (k, v) in res.headers() {
        response_builder = response_builder.header(k, v);
    }

    Ok(response_builder.body(Body::from_stream(res.bytes_stream()))?)
}
