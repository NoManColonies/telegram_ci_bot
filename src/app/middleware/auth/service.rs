use crate::app::util::error::ServiceError;
use axum::{body::BoxBody, response::IntoResponse};
use futures::future::{BoxFuture, FutureExt as _};
use hyper::Body;
use sqlx::{query, Pool, Sqlite};
use tower::Service;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SessionMiddleware<S> {
    pub inner: S,
}

#[derive(Debug, Clone)]
pub struct SessionContainer(pub Option<Session>);

#[derive(Debug, Clone)]
pub struct Session {
    pub sid: String,
}

impl<S> Service<hyper::Request<Body>> for SessionMiddleware<S>
where
    S: Service<hyper::Request<Body>, Response = hyper::Response<BoxBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: hyper::Request<Body>) -> Self::Future {
        // This is necessary because tonic internally uses `tower::buffer::Buffer`.
        // See https://github.com/tower-rs/tower/issues/547#issuecomment-767629149
        // for details on why this is necessary
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        async move {
            if let Err(e) = inspect_request_metadata(&mut req)
                .await
                .map_err(|e| e.into_response())
            {
                return Ok(e);
            }
            insert_empty_extension(&mut req);

            inner.call(req).await
        }
        .boxed()
    }
}

fn into_service_error<T, E>(error: E) -> Result<T, ServiceError>
where
    E: Into<ServiceError>,
{
    Err(error.into())
}

fn insert_empty_extension(req: &mut hyper::Request<Body>) {
    let extension = req.extensions_mut();

    if extension.get::<SessionContainer>().is_none() {
        extension.insert(SessionContainer(None));
    }
}

async fn inspect_request_metadata(req: &mut hyper::Request<Body>) -> Result<(), ServiceError> {
    let session = req
        .headers()
        .get("Authorization")
        .map(|header| header.to_str().map(|header| header.to_string()));

    let sqlite_pool = {
        let extension = req.extensions();

        extension.get::<Pool<Sqlite>>()
    }
    .cloned();

    match (session, sqlite_pool) {
        (Some(Ok(sid)), Some(sqlite_pool)) => {
            let record = query!(
                r#"
                SELECT * from main.repos
                WHERE id = ?
                "#,
                sid
            )
            .fetch_one(&sqlite_pool)
            .await;

            match record.map(|record| Uuid::parse_str(&record.id)) {
                Ok(Ok(_)) => {
                    let extension = req.extensions_mut();

                    extension.insert(SessionContainer(Some(Session { sid })));

                    Ok(())
                }
                Ok(Err(e)) => into_service_error(e)?,
                // Ok(None) => box_into_error(ServiceError::BadCredential)?,
                Err(e) => into_service_error(e)?,
            }
        }
        (Some(Err(e)), _) => into_service_error(e)?,
        (_, None) => into_service_error(ServiceError::MiddlewareNotSet("sqlite_pool"))?,
        // (None, _) => box_into_error(GeekyRepercussion::HttpHeaderNotFound)?,
        (None, _) => Ok(()),
    }
}
