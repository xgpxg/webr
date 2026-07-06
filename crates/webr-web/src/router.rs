use std::sync::Arc;

/// 路由注册 trait，由 `#[controller]` 宏自动实现
pub trait IntoRoutes {
    /// 返回当前 controller 注册的所有路由
    fn routes(self: Arc<Self>) -> axum::Router;
}

/// 框架路由，内部委托 `axum::Router`
pub struct WebrRouter {
    /// 底层 axum 路由实例
    inner: axum::Router,
}

impl WebrRouter {
    /// 创建空路由实例
    pub fn new() -> Self {
        Self {
            inner: axum::Router::new(),
        }
    }

    /// 合并 controller 路由
    pub fn merge_controller<C: IntoRoutes + 'static>(&mut self, controller: Arc<C>) {
        let routes = controller.routes();
        self.inner = std::mem::take(&mut self.inner).merge(routes);
    }

    /// 合并外部 `axum::Router`
    pub fn merge_axum_router(&mut self, router: axum::Router) {
        self.inner = std::mem::take(&mut self.inner).merge(router);
    }

    /// 消费自身，返回内部 `axum::Router`
    pub fn into_axum_router(self) -> axum::Router {
        self.inner
    }
}

impl Clone for WebrRouter {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl Default for WebrRouter {
    fn default() -> Self {
        Self::new()
    }
}
