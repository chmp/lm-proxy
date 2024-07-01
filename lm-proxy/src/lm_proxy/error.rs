use axum::response::{IntoResponse, Response};
use http::StatusCode;
use tracing::error;

pub type ServerResult<T, E = ServerError> = std::result::Result<T, E>;

pub struct ServerError(anyhow::Error);

impl std::fmt::Debug for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error({:?})", self.0)
    }
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:}", self.0)
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        // TODO: use callbacks, instead of conversion to log the error?
        error!(error = %self, "internal server error");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

impl<E: Into<anyhow::Error>> From<E> for ServerError {
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
