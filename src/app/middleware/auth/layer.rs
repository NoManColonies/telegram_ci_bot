use super::service::SessionMiddleware;
use tower::Layer;

#[derive(Debug, Clone)]
pub struct SessionLayer;

impl<S> Layer<S> for SessionLayer {
    type Service = SessionMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SessionMiddleware { inner }
    }
}
