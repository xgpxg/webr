use std::any::TypeId;

use crate::context::{ApplicationContext, FactoryFn};
use crate::error::FrameworkError;
use std::any::Any;

/// 所有托管组件必须实现的 trait，由 #[controller] / #[component] 宏自动 derive
pub trait Component: Any + Send + Sync + 'static {
    /// 组件类型名称，用于日志与调试
    fn component_name() -> &'static str;
}

/// 组件注册描述符。
/// 由 `#[component]` 宏生成并传递给 `ApplicationContext::register()`。
pub struct ComponentRegistration<E: std::error::Error + Send + Sync + 'static> {
    /// 组件类型
    pub type_id: TypeId,
    /// 组件名称
    pub name: &'static str,
    /// 依赖列表：每个元素为 `(依赖TypeId, 依赖名)`，名用于缺失依赖时的错误提示
    pub dependencies: Vec<(TypeId, &'static str)>,
    /// 工厂函数：从 ApplicationContext 创建组件实例
    pub factory: FactoryFn<E>,
}

/// inventory 注册的组件条目，由 `#[component]` / `#[controller]` 宏通过
/// `inventory::submit!` 提交，启动时由 `inventory::iter::<ComponentEntry>()` 收集。
pub struct ComponentEntry {
    /// 将组件注册到 IoC 容器
    pub register: fn(&mut ApplicationContext<FrameworkError>),
}

inventory::collect!(ComponentEntry);

/// inventory 注册的配置条目，由 `#[config]` 宏通过
/// `inventory::submit!` 提交，启动时由 `AppBuilder::build()` 收集。
pub struct ConfigEntry {
    /// 解析 TOML 根节点并将配置类型注册到 IoC 容器
    pub register:
        fn(&toml::Value, &mut ApplicationContext<FrameworkError>) -> Result<(), FrameworkError>,
}

inventory::collect!(ConfigEntry);
