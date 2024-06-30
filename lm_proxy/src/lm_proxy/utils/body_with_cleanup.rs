use std::{
    pin::Pin,
    task::{Context, Poll},
};

use http_body::{Body, Frame};
use pin_project::{pin_project, pinned_drop};

pub fn add_cleanup_to_body<F: FnOnce() + Send + Sync + 'static>(
    target: &mut axum::body::Body,
    cleanup: F,
) {
    let body = std::mem::take(target);
    let body = BodyWithCleanup::new(body, cleanup);
    let body = axum::body::Body::new(body);

    let body = std::mem::replace(target, body);
    let _ = body;
}

/// A [Body] with additional cleanup logic at the end of stream (or drop)
#[pin_project(PinnedDrop)]
pub struct BodyWithCleanup<B, F>
where
    F: FnOnce(),
{
    #[pin]
    body: B,
    cleanup: Option<F>,
}

impl<B, F> BodyWithCleanup<B, F>
where
    F: FnOnce(),
{
    pub fn new(body: B, cleanup: F) -> Self {
        Self {
            body,
            cleanup: Some(cleanup),
        }
    }
}

impl<B, F> Body for BodyWithCleanup<B, F>
where
    B: Body,
    F: FnOnce(),
{
    type Data = B::Data;
    type Error = B::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.project();
        match this.body.poll_frame(cx) {
            Poll::Ready(None) => {
                if let Some(cleanup) = this.cleanup.take() {
                    cleanup();
                }
                Poll::Ready(None)
            }
            res => res,
        }
    }
}

#[pinned_drop]
impl<B, F: FnOnce()> PinnedDrop for BodyWithCleanup<B, F> {
    fn drop(self: Pin<&mut Self>) {
        let this = self.project();
        if let Some(cleanup) = this.cleanup.take() {
            cleanup();
        }
    }
}
