use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::component::ComponentEntry;
use crate::config::{ConfigEntry, ConfigLoader};
use crate::context::ApplicationContext;
use crate::error::Error;
use crate::middleware::{Middleware, Next, ScopedMiddleware, UnifiedResponse};
use crate::router::WebrRouter;

/// 生命周期回调类型：接收 IoC 容器引用，返回异步结果
type LifecycleCallback = Box<
    dyn Fn(&ApplicationContext) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send>>
        + Send
        + Sync,
>;

/// 应用构建器。
///
/// 通过链式 API 配置服务器参数、中间件和组件，调用 [`run`](Self::run) 启动服务。
/// 组件通过 `inventory` 自动扫描注册，无需手动装配。
pub struct AppBuilder {
    /// IoC 容器，管理组件生命周期与依赖解析
    context: ApplicationContext,
    /// 路由收集器，汇总所有 controller 的路由
    router: WebrRouter,
    /// 全局中间件链（对所有路由生效）
    middlewares: Vec<Arc<dyn Middleware>>,
    /// 路径范围中间件（按匹配/排除规则生效）
    scoped_middlewares: Vec<Arc<ScopedMiddleware>>,
    /// HTTP 监听端口，默认 8080
    port: u16,
    /// HTTP 监听地址，默认 `0.0.0.0`
    host: String,
    /// 请求体大小上限（字节），默认 2MB
    max_body_size: usize,
    /// 是否已完成构建（防止重复 build）
    built: bool,
    /// 配置加载器（持有原始 TOML 配置及 profile 信息）
    config: ConfigLoader,
    /// 启动就绪回调（服务开始接受连接后执行）
    on_ready_callbacks: Vec<LifecycleCallback>,
    /// 关闭前回调（收到关闭信号后、连接排空前执行）
    on_shutdown_callbacks: Vec<LifecycleCallback>,
}

impl AppBuilder {
    pub fn new() -> Self {
        // 加载配置文件
        let config = ConfigLoader::load().expect("Failed to load configuration");

        // 初始化 tracing
        let base_filter = config
            .raw()
            .get("log")
            .and_then(|l| l.get("level"))
            .and_then(|v| v.as_str())
            .unwrap_or("info");
        // 由于 `webr-db` 中已经对 sql 日志做了处理，所以在此处关闭 sqlx 的sql日志日志打印
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

        // 应用 server 配置
        let server = builder.config.server_config();
        builder.host = server.host;
        builder.port = server.port;
        builder.max_body_size = server.max_body_size;

        builder
    }

    // ─── 配置 API ──────────────────────────────────────────

    /// 设置监听端口（默认 8080）。
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// 设置监听地址（默认 `0.0.0.0`）。
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    /// 获取 IoC 容器的可变引用（用于手动注册等高级场景）。
    pub fn context_mut(&mut self) -> &mut ApplicationContext {
        &mut self.context
    }

    /// 获取 IoC 容器的引用。
    pub fn context(&self) -> &ApplicationContext {
        &self.context
    }

    /// 获取配置加载器。
    pub fn config(&self) -> &ConfigLoader {
        &self.config
    }

    /// 手动注册组件实例（适用于配置对象等无法使用宏的类型）。
    pub fn provide<T: crate::component::Component>(&mut self, instance: T) -> Result<(), Error> {
        self.context.provide(instance)
    }

    /// 添加全局中间件。
    pub fn middleware<M: Middleware>(&mut self, middleware: M) {
        self.middlewares.push(Arc::new(middleware));
    }

    /// 启用统一响应包装。
    ///
    /// 2xx JSON 响应自动包装为 `{"code": 200, "message": "success", "data": ...}`，
    /// 非 2xx 和非 JSON 响应不受影响。
    pub fn unified_response(&mut self) {
        self.middlewares.push(Arc::new(UnifiedResponse));
    }

    /// 添加路径范围中间件，仅对匹配的路由生效。
    ///
    /// 模式语法：`/api/**` 前缀匹配，`/health` 精确匹配。
    pub fn middleware_for<M: Middleware>(&mut self, pattern: &'static str, middleware: M) {
        self.scoped_middlewares.push(Arc::new(ScopedMiddleware::new(
            pattern,
            Arc::new(middleware),
        )));
    }

    /// 添加排除路径中间件，对**不匹配**的路由生效。
    pub fn middleware_except<M: Middleware>(&mut self, pattern: &'static str, middleware: M) {
        self.scoped_middlewares
            .push(Arc::new(ScopedMiddleware::exclude(
                pattern,
                Arc::new(middleware),
            )));
    }

    /// 注册启动就绪回调，在服务开始接受连接**后**执行。
    ///
    /// 回调接收 IoC 容器引用，可解析已构建的组件执行初始化逻辑。
    /// 任一回调返回 `Err` 时服务不会启动。
    ///
    /// # Example
    /// ```rust
    /// app.on_ready(|ctx| async move {
    ///     let service = ctx.resolve::<MyService>()?;
    ///     service.warm_up().await;
    ///     tracing::info!("Cache warmed up");
    ///     Ok(())
    /// });
    /// ```
    pub fn on_ready<F, Fut>(&mut self, callback: F)
    where
        F: Fn(&ApplicationContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), Error>> + Send + 'static,
    {
        self.on_ready_callbacks
            .push(Box::new(move |ctx| Box::pin(callback(ctx))));
    }

    /// 注册关闭前回调，在收到关闭信号**后**执行。
    ///
    /// 适用于资源清理、通知下游服务等场景。
    /// 回调失败仅记录日志，不阻止关闭流程。
    ///
    /// # Example
    /// ```rust
    /// app.on_shutdown(|ctx| async move {
    ///     let pool = ctx.resolve::<DbPool>()?;
    ///     pool.close().await;
    ///     tracing::info!("Database connection closed");
    ///     Ok(())
    /// });
    /// ```
    pub fn on_shutdown<F, Fut>(&mut self, callback: F)
    where
        F: Fn(&ApplicationContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), Error>> + Send + 'static,
    {
        self.on_shutdown_callbacks
            .push(Box::new(move |ctx| Box::pin(callback(ctx))));
    }

    // ─── 生命周期 API ──────────────────────────────────────

    /// 构建应用：扫描组件 → 拓扑排序实例化 → 挂载路由 → 打印路由表。
    ///
    /// 幂等：重复调用无副作用，[`run`](Self::run) 会自动调用。
    pub fn build(&mut self) -> Result<(), Error> {
        if self.built {
            return Ok(());
        }

        // 注册 #[config] 标注的自定义配置类型
        let config_entries: Vec<&ConfigEntry> = inventory::iter::<ConfigEntry>().collect();
        for entry in &config_entries {
            (entry.register)(self.config.raw(), &mut self.context)?;
        }

        // 收集 inventory 中所有自动注册的组件
        let mut entries: Vec<&ComponentEntry> = Vec::new();
        for entry in inventory::iter::<ComponentEntry>() {
            (entry.register)(&mut self.context);
            entries.push(entry);
        }

        // 拓扑排序并实例化所有组件
        self.context.build()?;

        // 挂载 controller 路由，收集路由元数据
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

    /// 启动 HTTP 服务（内部自动调用 [`build`](Self::build)）。
    ///
    /// 执行顺序：build → on_ready → 绑定监听 → 接受连接 → (Ctrl+C) → on_shutdown → 退出
    pub async fn run(mut self) -> Result<(), Error> {
        self.build()?;

        // 包装 IoC 容器为 Arc，供生命周期回调共享访问
        let context = Arc::new(self.context);

        // 执行启动就绪回调（任一失败则不启动服务）
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

        // 优雅关闭：监听 Ctrl+C → 执行 on_shutdown 回调 → 等待连接排空
        let shutdown_context = Arc::clone(&context);
        axum::serve(listener, axum_router)
            .with_graceful_shutdown(async move {
                tokio::signal::ctrl_c().await.ok();
                tracing::info!("Shutdown signal received, draining connections...");

                // 执行关闭前回调（失败仅记录日志，不阻止关闭）
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

/// 将中间件应用到 axum Router。
///
/// 应用顺序：scoped（内层）→ global（外层）。
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
