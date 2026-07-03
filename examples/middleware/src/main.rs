use webr::prelude::*;
use webr::{async_trait::async_trait, axum, CorsMiddleware, Middleware, Next, PanicRecovery, Error};

// ─── 自定义中间件：鉴权 ──────────────────────────────────

/// 鉴权中间件：检查 X-Token 请求头
pub struct RequireAuth;

#[async_trait]
impl Middleware for RequireAuth {
    async fn handle(
        &self,
        request: axum::extract::Request,
        next: Next,
    ) -> axum::response::Response {
        if request.headers().get("X-Token").is_some() {
            next.run(request).await
        } else {
            (StatusCode::UNAUTHORIZED, "Missing X-Token header").into_response()
        }
    }
}

// ─── 自定义中间件：请求日志 ──────────────────────────────

/// 请求耗时统计中间件
pub struct RequestTimer;

#[async_trait]
impl Middleware for RequestTimer {
    async fn handle(
        &self,
        request: axum::extract::Request,
        next: Next,
    ) -> axum::response::Response {
        let start = std::time::Instant::now();
        let method = request.method().clone();
        let uri = request.uri().clone();
        
        let response = next.run(request).await;
        
        let duration = start.elapsed();
        println!("[{}] {} {} - {}ms", 
            response.status().as_u16(),
            method,
            uri,
            duration.as_millis()
        );
        
        response
    }
}

// ─── Controller ─────────────────────────────────────────

#[controller]
pub struct DemoController;

#[controller]
impl DemoController {
    /// 公开接口，无需鉴权
    #[get("/")]
    async fn index(&self) -> &'static str {
        "Middleware Example - Try /api/protected (requires X-Token header)"
    }

    /// 公开接口
    #[get("/public")]
    async fn public(&self) -> webr::Json<serde_json::Value> {
        webr::Json(serde_json::json!({
            "message": "This is a public endpoint",
            "auth_required": false
        }))
    }

    /// Panic 测试：PanicRecovery 中间件会捕获并返回 500
    #[get("/panic")]
    async fn trigger_panic(&self) -> &'static str {
        panic!("Something went terribly wrong!");
    }
}

#[controller]
pub struct ApiController;

#[controller(prefix = "/api")]
impl ApiController {
    /// 受保护接口：需要 X-Token 请求头
    #[get("/protected")]
    async fn protected(&self) -> webr::Json<serde_json::Value> {
        webr::Json(serde_json::json!({
            "message": "You have accessed a protected endpoint!",
            "auth_required": true
        }))
    }

    #[get("/data")]
    async fn data(&self) -> webr::Json<serde_json::Value> {
        webr::Json(serde_json::json!({
            "items": [
                {"id": 1, "name": "Item A"},
                {"id": 2, "name": "Item B"},
                {"id": 3, "name": "Item C"}
            ]
        }))
    }
}

// ─── 启动入口 ──────────────────────────────────────────

#[webr::main]
async fn main(app: &mut webr::AppBuilder) -> Result<(), Error> {
    // 全局中间件
    
    // Panic 恢复：捕获 panic，返回 500 而不是进程崩溃
    app.middleware(PanicRecovery);
    
    // 请求耗时统计
    app.middleware(RequestTimer);
    
    // 全局 CORS 中间件
    app.middleware(
        CorsMiddleware::new()
            .allow_origin("*")
            .allow_methods(["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allow_headers(["Content-Type", "X-Token"])
            .max_age(3600),
    );
    
    // 只对 /api/** 路由启用鉴权
    app.middleware_for("/api/**", RequireAuth);
    
    // 统一响应格式
    app.unified_response();
    
    Ok(())
}
