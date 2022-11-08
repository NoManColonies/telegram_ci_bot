use crate::app::{
    config::{database::init_sqlite, task::spawn_with_name},
    middleware::auth::layer::SessionLayer,
    service::{
        bot::{
            handler::{config_repo, invalid_command, receive_status, start},
            state::{BotState, RepoCommand},
        },
        root::{root_failure_handler, root_handler},
        status::update_status,
    },
};
use axum::{
    extract::Extension,
    routing::{get, post, put},
    Router,
};
use hyper::{Body, Method};
use sentry_tower::{NewSentryLayer, SentryHttpLayer};
use sentry_tracing::EventFilter;
use std::{env::var, net::SocketAddr, sync::Arc, time::Duration};
use teloxide::{
    dispatching::dialogue::{serializer::Bincode, ErasedStorage, RedisStorage, Storage},
    prelude::*,
};
use tokio::{signal, sync::Notify};
use tower::ServiceBuilder;
use tower_http::{
    classify::ServerErrorsFailureClass,
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::{debug, error, field, info, info_span, Span};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_futures::Instrument;
use tracing_log::LogTracer;
use tracing_subscriber::{
    layer::SubscriberExt,
    {EnvFilter, Registry},
};

#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

lazy_static::lazy_static! {
    static ref APP_NAME: &'static str = env!("CARGO_PKG_NAME");
    static ref APP_VERSION: &'static str = env!("CARGO_PKG_VERSION");
    static ref APP_URL: String = var("APP_URL").expect("expect an APP_URL to be set. app url define internal IP address for application to bind and is usually 0.0.0.0");
    static ref APP_PORT: String = var("APP_PORT").expect("expect an APP_PORT to be set. app port define virtual port for app to bind to");
    static ref SENTRY_URL: String = var("SENTRY_URL").expect("expect SENTRY_URL to be set");
    static ref DATABASE_URL: String = var("DATABASE_URL").expect("expect DATABASE_URL to be set");
}

mod app;

type StateStorage = std::sync::Arc<ErasedStorage<BotState>>;

#[tokio::main]
async fn main() {
    LogTracer::init().expect("expect log tracer to complete the setup process");
    dotenv::dotenv().expect("expect a .env file and valid syntax");
    let name = &*APP_NAME;
    let version = &*APP_VERSION;
    // setup sentry DSN from env if `sentry-io` feature is enabled
    let sentry_url = &*SENTRY_URL.to_owned();
    // setup sentry client hub and bind to the main thread
    let sentry_guard = sentry::init((
        sentry_url, // <- the DSN key
        sentry::ClientOptions {
            release: sentry::release_name!(), // <- the application/service name that show up in sentry dashboard
            integrations: vec![
                Arc::new(sentry_backtrace::AttachStacktraceIntegration), // <- attach stacktrace if our service crashed unexpectedly
                Arc::new(sentry_backtrace::ProcessStacktraceIntegration), // <- process stacktrace in case our service crashed
            ],
            session_mode: sentry::SessionMode::Request, // <- setup a session mode to be `per request`
            auto_session_tracking: true,                // <- made session tracking automatic
            ..Default::default()                        // <- leave other options to default
        },
    ));

    #[cfg(not(feature = "stdout"))]
    let file_appender = tracing_appender::rolling::hourly("/tmp/avalue-ci-bot/log", "hourly.log");
    #[cfg(not(feature = "stdout"))]
    let (non_blocking_writer, _guard) = tracing_appender::non_blocking(file_appender);

    #[cfg(feature = "stdout")]
    let (non_blocking_writer, _non_blocking_writer_guard) =
        tracing_appender::non_blocking(std::io::stdout());

    let bunyan_formatting_layer =
        BunyanFormattingLayer::new(format!("{}-{}", name, version), non_blocking_writer);
    let sentry_layer = sentry_tracing::layer().event_filter(|md| match *md.level() {
        tracing::Level::ERROR | tracing::Level::WARN => EventFilter::Breadcrumb,
        _ => EventFilter::Ignore,
    });

    let filter_layer = EnvFilter::new("INFO");
    let subscriber = Registry::default()
        .with(filter_layer)
        .with(JsonStorageLayer)
        .with(bunyan_formatting_layer)
        .with(sentry_layer);
    tracing::subscriber::set_global_default(subscriber)
        .expect("expect a tracing subscriber to complete the setup process");

    info!("Starting avalue ci bot...");

    let storage: StateStorage = RedisStorage::open("redis://127.0.0.1:6379", Bincode)
        .await
        .expect("expect a redis connection to be made successfully.")
        .erase();

    let sqlite_pool = init_sqlite()
        .await
        .expect("expect sqlite pool to be setup successfully");

    sqlx::migrate!()
        .run(&sqlite_pool)
        .await
        .expect("expect a migration to complete successfully");

    let bot = Bot::from_env();

    let teloxide_handler = spawn_with_name(
        {
            let root_span = info_span!("teloxide");
            let sqlite_pool = sqlite_pool.clone();
            let bot = bot.clone();
            async move {
                // teloxide::repl(bot, |bot: Bot, msg: Message| async move {
                //     info!("receiving a message with id: {:?}", msg.chat.id.0);
                //     bot.send_dice(msg.chat.id).await?;
                //     // info!("{}", ChatId(-854492643));
                //     Ok(())
                // })
                // .await
                Dispatcher::builder(
                    bot,
                    Update::filter_message()
                        .enter_dialogue::<Message, ErasedStorage<BotState>, BotState>()
                        .branch(dptree::case![BotState::Start].endpoint(start))
                        .branch(dptree::case![BotState::ConfigMode].endpoint(config_repo))
                        .branch(
                            dptree::case![BotState::ReceiverMode(name, key)]
                                .branch(
                                    dptree::entry()
                                        .filter_command::<RepoCommand>()
                                        .endpoint(receive_status),
                                )
                                .branch(dptree::endpoint(invalid_command)),
                        ),
                )
                .dependencies(dptree::deps![storage, sqlite_pool])
                .enable_ctrlc_handler()
                .build()
                .dispatch()
                .await
            }
            .instrument(root_span)
        },
        "teloxide",
    );
    // thread safe application shutdown signal notifier
    let shutdown_signal_notifier = Arc::new(Notify::new());
    // graceful shutdown handler
    let shutdown_handler = spawn_with_name(
        {
            let root_span = info_span!("shutdown interceptor");
            let shutdown_signal_notifier = Arc::clone(&shutdown_signal_notifier);

            async move {
                debug!("waiting for ctrl-c signal...");
                // wait for ctrl-c signal
                signal::ctrl_c()
                    .await
                    .expect("expect ctrl-c signal to be successfully received");
                debug!("received ctrl-c signal");

                // notify all client about application shutting down
                shutdown_signal_notifier.notify_waiters();
            }
            .instrument(root_span)
        },
        "shutdown interceptor",
    );
    // parse socket address from env
    let addr = format!("{}:{}", *APP_URL, *APP_PORT)
        .parse::<SocketAddr>()
        .expect("expect a successfully parsed url");

    let app =
        Router::new()
            .route("/", get(root_handler))
            .route("/", post(root_failure_handler))
            .route("/status", put(update_status))
            .layer(
                ServiceBuilder::new()
                    .layer(
                        TraceLayer::new_for_http()
                            .make_span_with(|request: &http::Request<Body>| {
                                tracing::info_span!(
                                    "request",
                                    method = %request.method(),
                                    uri = request.uri().path(),
                                    version = ?request.version(),
                                    headers = ?request.headers(),
                                    latency = field::Empty,
                                    status = field::Empty
                                )
                            })
                            .on_request(|request: &http::Request<Body>, _span: &Span| {
                                info!("INCOMING {} {}", request.method(), request.uri().path())
                            })
                            .on_response(
                                |response: &http::Response<_>, 
                                latency: Duration, 
                                _span: &Span| {
                                    info!(latency = ?latency, status = %response.status(), "response generated");
                            })
                            .on_failure(
                                |error: ServerErrorsFailureClass,
                                 latency: Duration,
                                 _span: &Span| {
                                    error!(latency = ?latency, "Service threw an exception: {:?}", error)
                                },
                            ),
                    )
                    .layer(NewSentryLayer::new_from_top())
                    .layer(SentryHttpLayer::with_transaction())
                    .layer(CompressionLayer::new())
                    .layer(
                        CorsLayer::new()
                            .allow_methods([
                                Method::GET, 
                                Method::POST, 
                                Method::PUT
                            ])
                            .allow_origin(Any),
                    )
                    .layer(TimeoutLayer::new(Duration::from_secs(30)))
                    .layer(Extension(sqlite_pool))
                    .layer(Extension(bot))
                    .layer(SessionLayer),
            );
    // .fallback(unknown_route_handler);

    let axum_handler = spawn_with_name(
        {
            let root_span = info_span!("axum server");
            let shutdown_signal_notifier = Arc::clone(&shutdown_signal_notifier);

            async move {
                axum::Server::bind(&addr)
                    .serve(app.into_make_service())
                    .with_graceful_shutdown(
                        async move { shutdown_signal_notifier.notified().await },
                    )
                    .await
            }
            .instrument(root_span)
        },
        "axum server",
    );
    let _ = tokio::join!(teloxide_handler, axum_handler, shutdown_handler);

    info!("performing graceful shutdown which may take up to 10 seconds... or ctrl-c to force shutdown");
    tokio::select! {
        _ = tokio::task::spawn_blocking(move || sentry_guard.flush(Some(Duration::from_secs(10)))) => {
            debug!("exiting...");
        }
        _ = signal::ctrl_c() => {
            debug!("force shutting down...");
        }
    }
}
