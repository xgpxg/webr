use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use webr_core::config::ConfigLoader;
use webr_core::context::ApplicationContext;
use webr_core::error::FrameworkError;

use crate::component::{ComponentEntry, ConfigEntry};
use crate::error::Error;
use crate::middleware::{Middleware, Next, ScopedMiddleware, UnifiedResponse};
use crate::router::WebrRouter;

/// Lifecycle callback type: receives IoC container reference, returns async result
type LifecycleCallback = Box<
    dyn Fn(&ApplicationContext<Error>) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send>>
        + Send
        + Sync,
>;

/// Application builder.
///
/// Configures server parameters, middleware, and components via chainable API.
/// Call [`run`](Self::run) to start the HTTP server.
/// Components are auto-scanned and registered via `inventory`.
pub struct AppBuilder {
    /// IoC container managing component lifecycle
    context: ApplicationContext<Error>,
    /// Route collector aggregating all controller routes
    router: WebrRouter,
    /// Global middleware chain (applied to all routes)
    middlewares: Vec<Arc<dyn Middleware>>,
    /// Scoped middleware (applied by match/exclude patterns)
    scoped_middlewares: Vec<Arc<ScopedMiddleware>>,
    /// HTTP listen port, default 8080
    port: u16,
    /// HTTP listen address, default `0.0.0.0`
    host: String,
    /// Max request body size in bytes, default 2MB
    max_body_size: usize,
    /// Whether build() has been called
    built: bool,
    /// Configuration loader (holds raw TOML and profile info)
    config: ConfigLoader,
    /// Callbacks executed after server starts accepting connections
    on_ready_callbacks: Vec<LifecycleCallback>,
    /// Callbacks executed before graceful shutdown
    on_shutdown_callbacks: Vec<LifecycleCallback>,
}

impl AppBuilder {
    pub fn new() -> Self {
        // Load configuration files
        let config = ConfigLoader::load().expect("Failed to load configuration");

        // Initialize tracing
        let base_filter = config
            .raw()
            .get("log")
            .and_then(|l| l.get("level"))
            .and_then(|v| v.as_str())
            .unwrap_or("info");
        // sqlx SQL logging is handled in webr-db, disable it here
        let filter = format!("{},sqlx=info", base_filter);
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_new(&filter)
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,sqlx=off")),
            )
            .try_init();

        tracing::info!("Starting WebR application...");
        tracing::info!(
            "Configuration loaded: profile={}, files=[{}]",
            config.profile(),
            config.files_loaded().join(", ")
        );

        let mut builder = Self {
            context: ApplicationContext::new(),
            router: WebrRouter::new(),
            middlewares: Vec::new(),
            scoped_middlewares: Vec::new(),
            port: 8080,
            host: "0.0.0.0".into(),
            max_body_size: 2 * 1024 * 1024,
            built: false,
            config,
            on_ready_callbacks: Vec::new(),
            on_shutdown_callbacks: Vec::new(),
        };

        // Apply server config
        let server = builder.config.server_config();
        builder.host = server.host;
        builder.port = server.port;
        builder.max_body_size = server.max_body_size;

        builder
    }

    // ─── Configuration API ──────────────────────────────────────────

    /// Set listen port (default 8080).
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set listen address (default `0.0.0.0`).
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    /// Get mutable reference to the IoC container.
    pub fn context_mut(&mut self) -> &mut ApplicationContext<Error> {
        &mut self.context
    }

    /// Get reference to the IoC container.
    pub fn context(&self) -> &ApplicationContext<Error> {
        &self.context
    }

    /// Get the configuration loader.
    pub fn config(&self) -> &ConfigLoader {
        &self.config
    }

    /// Manually register a component instance (for config objects etc. that can't use macros).
    ///
    /// Returns `FrameworkError` on duplicate registration; use `?` with `webr::Result`
    /// which auto-converts via `From<FrameworkError> for Error`.
    pub fn provide<T: webr_core::component::Component>(
        &mut self,
        instance: T,
    ) -> Result<(), FrameworkError> {
        self.context.provide(instance)
    }

    /// Add global middleware.
    pub fn middleware<M: Middleware>(&mut self, middleware: M) {
        self.middlewares.push(Arc::new(middleware));
    }

    /// Enable unified response wrapping.
    ///
    /// 2xx JSON responses are wrapped as `{"code": 200, "message": "success", "data": ...}`,
    /// non-2xx and non-JSON responses are unaffected.
    pub fn unified_response(&mut self) {
        self.middlewares.push(Arc::new(UnifiedResponse));
    }

    /// Add scoped middleware, only applied to matching route patterns.
    ///
    /// Pattern syntax: `/api/**` prefix match, `/health` exact match.
    pub fn middleware_for<M: Middleware>(&mut self, pattern: &'static str, middleware: M) {
        self.scoped_middlewares.push(Arc::new(ScopedMiddleware::new(
            pattern,
            Arc::new(middleware),
        )));
    }

    /// Add exclusion middleware, applied to routes that do **not** match the pattern.
    pub fn middleware_except<M: Middleware>(&mut self, pattern: &'static str, middleware: M) {
        self.scoped_middlewares
            .push(Arc::new(ScopedMiddleware::exclude(
                pattern,
                Arc::new(middleware),
            )));
    }

    /// Register a startup-ready callback, executed **after** the server starts accepting connections.
    ///
    /// Callback receives the IoC container reference for resolving built components.
    /// If any callback returns `Err`, the server will not start.
    pub fn on_ready<F, Fut>(&mut self, callback: F)
    where
        F: Fn(&ApplicationContext<Error>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), Error>> + Send + 'static,
    {
        self.on_ready_callbacks
            .push(Box::new(move |ctx| Box::pin(callback(ctx))));
    }

    /// Register a pre-shutdown callback, executed **after** the shutdown signal is received.
    ///
    /// Suitable for resource cleanup and downstream notification.
    /// Callback failure only logs an error, does not prevent shutdown.
    pub fn on_shutdown<F, Fut>(&mut self, callback: F)
    where
        F: Fn(&ApplicationContext<Error>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), Error>> + Send + 'static,
    {
        self.on_shutdown_callbacks
            .push(Box::new(move |ctx| Box::pin(callback(ctx))));
    }

    // ─── Lifecycle API ──────────────────────────────────────

    /// Build the application: scan components → topological sort → mount routes → print route table.
    ///
    /// Idempotent: repeated calls are no-ops. [`run`](Self::run) calls this automatically.
    pub async fn build(&mut self) -> Result<(), Error> {
        if self.built {
            return Ok(());
        }

        // Register #[config] annotated custom config types
        let config_entries: Vec<&ConfigEntry> = inventory::iter::<ConfigEntry>().collect();
        for entry in &config_entries {
            (entry.register)(self.config.raw(), &mut self.context)?;
        }

        // Collect all auto-registered component entries
        let mut entries: Vec<&ComponentEntry> = Vec::new();
        for entry in inventory::iter::<ComponentEntry>() {
            (entry.register)(&mut self.context);
            entries.push(entry);
        }

        // Topological sort and instantiate all components
        self.context.build().map_err(Error::from)?;

        // Mount controller routes and collect route metadata
        let mut all_routes: Vec<(&str, &str, &str)> = Vec::new();
        for entry in &entries {
            if let Some(mount) = entry.mount {
                mount(&self.context, &mut self.router)?;
            }
            all_routes.extend_from_slice(entry.routes);
        }

        if !all_routes.is_empty() {
            let max_method = all_routes
                .iter()
                .map(|(m, _, _)| m.len())
                .max()
                .unwrap_or(0);
            let max_path = all_routes
                .iter()
                .map(|(_, p, _)| p.len())
                .max()
                .unwrap_or(0);
            tracing::info!("Route mappings:");
            for (method, path, controller) in &all_routes {
                tracing::info!(
                    "  {:<width_m$} {:<width_p$} → {}",
                    method,
                    path,
                    controller,
                    width_m = max_method,
                    width_p = max_path
                );
            }
        }

        if !self.scoped_middlewares.is_empty() {
            tracing::info!("Scoped middlewares:");
            for sm in &self.scoped_middlewares {
                tracing::info!("  {} → {}", sm.pattern(), sm.middleware_name());
            }
        }

        self.built = true;
        Ok(())
    }

    /// Start the HTTP server (internally calls [`build`](Self::build)).
    ///
    /// Execution order: build → on_ready → bind → accept → (Ctrl+C) → on_shutdown → exit
    pub async fn run(mut self) -> Result<(), Error> {
        self.build().await?;

        // Wrap IoC container in Arc for shared access across lifecycle callbacks
        let context = Arc::new(self.context);

        // Execute startup-ready callbacks (any failure prevents server start)
        for callback in &self.on_ready_callbacks {
            callback(&context).await?;
        }

        let Self {
            router,
            middlewares,
            scoped_middlewares,
            host,
            port,
            max_body_size,
            on_shutdown_callbacks,
            ..
        } = self;
        let axum_router =
            apply_middlewares(router.into_axum_router(), middlewares, scoped_middlewares)
                .layer(axum::extract::DefaultBodyLimit::max(max_body_size));

        let addr = format!("{host}:{port}");
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .map_err(|e| Error::Internal(format!("Failed to bind {addr}: {e}")))?;

        tracing::info!("WebR started on http://{}", addr);

        // Graceful shutdown: Ctrl+C → on_shutdown callbacks → drain connections
        let shutdown_context = Arc::clone(&context);
        axum::serve(listener, axum_router)
            .with_graceful_shutdown(async move {
                tokio::signal::ctrl_c().await.ok();
                tracing::info!("Shutdown signal received, draining connections...");

                for callback in &on_shutdown_callbacks {
                    if let Err(e) = callback(&shutdown_context).await {
                        tracing::error!("on_shutdown callback error: {e}");
                    }
                }
            })
            .await
            .map_err(|e| Error::Internal(format!("Server error: {e}")))?;

        tracing::info!("WebR stopped gracefully.");
        Ok(())
    }
}

/// Apply middlewares to an axum Router.
///
/// Order: scoped (inner layer) → global (outer layer).
fn apply_middlewares(
    mut router: axum::Router,
    global: Vec<Arc<dyn Middleware>>,
    scoped: Vec<Arc<ScopedMiddleware>>,
) -> axum::Router {
    for sm in scoped {
        router = router.layer(axum::middleware::from_fn(
            move |req: axum::extract::Request, next: axum::middleware::Next| {
                let sm = Arc::clone(&sm);
                async move {
                    let path = req.uri().path();
                    if sm.matches(path) {
                        sm.middleware().handle(req, Next::new(next)).await
                    } else {
                        next.run(req).await
                    }
                }
            },
        ));
    }

    for mw in global {
        router = router.layer(axum::middleware::from_fn(
            move |req: axum::extract::Request, next: axum::middleware::Next| {
                let mw = Arc::clone(&mw);
                async move { mw.handle(req, Next::new(next)).await }
            },
        ));
    }

    router
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self::new()
    }
}
