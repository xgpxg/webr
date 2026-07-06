use proc_macro::TokenStream;

mod controller;
mod config;
mod entity;
mod error_derive;
mod main_macro;
mod route;
mod component;
mod sql_macro;
mod sql_parser;
mod tx;

/// 标记一个 struct 或 impl block 为 Controller。
/// - 在 struct 上：生成 Component 实现 + 构造函数 + 注册描述符
/// - 在 impl 上：解析路由注解，生成 handler 函数 + IntoRoutes 实现
#[proc_macro_attribute]
pub fn controller(attr: TokenStream, item: TokenStream) -> TokenStream {
    controller::expand_controller(attr.into(), item.into()).into()
}

/// 标记一个 struct 为 Component。
/// 生成 Component 实现 + 构造函数（自动注入 Inject<T> 字段）+ 注册描述符
#[proc_macro_attribute]
pub fn component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    component::expand_component(item.into()).into()
}

/// GET 路由注解（仅在 #[controller] impl 内生效）
#[proc_macro_attribute]
pub fn get(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// POST 路由注解
#[proc_macro_attribute]
pub fn post(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// PUT 路由注解
#[proc_macro_attribute]
pub fn put(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// DELETE 路由注解
#[proc_macro_attribute]
pub fn delete(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// PATCH 路由注解
#[proc_macro_attribute]
pub fn patch(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// 应用入口宏：将 async fn main 包装为 tokio runtime + WebR 启动逻辑
#[proc_macro_attribute]
pub fn main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    main_macro::expand_main(item.into()).into()
}

/// 配置类宏：将 struct 注册为可注入的配置类型，从 TOML 配置文件的指定 section 加载。
///
/// 用法：`#[config(prefix = "app")]` → 从 `[app]` 节加载配置
#[proc_macro_attribute]
pub fn config(attr: TokenStream, item: TokenStream) -> TokenStream {
    config::expand_config(attr.into(), item.into()).into()
}

/// 为自定义业务错误 enum 生成 HTTP 响应映射。
///
/// 变体必须加 `#[error(status = N)]` 或 `#[error(status = N, message = "...")]`。
/// 生成 `IntoResponse`（可直接作为返回类型）和 `From<Self> for Error`（支持 `?` 转换）。
#[proc_macro_derive(HttpError, attributes(error))]
pub fn derive_http_error(item: TokenStream) -> TokenStream {
    error_derive::expand_webr_error(syn::parse_macro_input!(item as syn::DeriveInput)).into()
}

/// 标记一个 struct 为数据库实体，自动生成 CRUD 方法和 sea-query Iden 枚举。
///
/// 用法：`#[entity(table = "users")]`
/// 字段属性：
/// - `#[column(pk)]` — 标记主键（必须）
/// - `#[column(name = "col")]` — 自定义列名映射（默认使用字段名）
/// - `#[column(pk, name = "col")]` — 两者组合
#[proc_macro_attribute]
pub fn entity(attr: TokenStream, item: TokenStream) -> TokenStream {
    entity::expand_entity(attr.into(), item.into())
        .unwrap_or_else(|e| e.into_compile_error())
        .into()
}

/// 在实体 impl 方法上标注 SQL 查询，支持 MyBatis 风格的动态标签。
///
/// 用法：`#[sql(r#"SELECT * FROM users WHERE id = #{id}"#)]`
/// 支持标签：`<if>`, `<where>`, `<set>`, `<foreach>`, `<choose>`, `<when>`, `<otherwise>`, `<trim>`
#[proc_macro_attribute]
pub fn sql(attr: TokenStream, item: TokenStream) -> TokenStream {
    sql_macro::expand_sql(attr.into(), item.into()).into()
}

/// 声明式事务注解：将 impl block 中的所有 `async fn` 方法包装在事务中。
///
/// - 自动 commit/rollback（`Result` 返回类型：Ok → commit，Err → rollback）
/// - REQUIRED 传播：嵌套调用时加入外层事务
/// - 默认使用 struct 的 `pool` 字段；可用 `#[tx(pool = "db_pool")]` 覆盖
#[proc_macro_attribute]
pub fn tx(attr: TokenStream, item: TokenStream) -> TokenStream {
    tx::expand_tx(attr.into(), item.into()).into()
}
