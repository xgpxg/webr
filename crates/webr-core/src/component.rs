use std::any::TypeId;

use crate::context::{ApplicationContext, FactoryFn};
use crate::error::Error;
use std::any::Any;

/// 所有托管组件必须实现的 trait，由 #[controller] / #[service] 宏自动 derive
pub trait Component: Any + Send + Sync + 'static {
    /// 组件类型名称，用于日志与调试
    fn component_name() -> &'static str;
}

/// 组件注册描述符。
/// 由 `#[component]` 宏生成并传递给 `ApplicationContext::register()`。
pub struct ComponentRegistration {
    /// 组件类型
    pub type_id: TypeId,
    /// 组件名称
    pub name: &'static str,
    /// 依赖的 TypeId 列表，用于拓扑排序
    pub dependencies: Vec<TypeId>,
    /// 工厂函数：从 ApplicationContext 创建组件实例
    pub factory: FactoryFn,
}

/// 路由挂载函数类型
pub type MountFn = fn(&ApplicationContext, &mut crate::router::WebrRouter) -> Result<(), Error>;

/// 路由描述符：(HTTP方法, 路径, 控制器名)
pub type RouteDescriptor = (&'static str, &'static str, &'static str);

/// 自动注册的组件条目，由 #[controller] / #[component] 宏通过 `inventory::submit!` 提交。
/// 启动时通过 `inventory::iter::<ComponentEntry>()` 自动收集。
pub struct ComponentEntry {
    /// 将组件注册到 IoC 容器
    pub register: fn(&mut ApplicationContext),
    /// 将 controller 路由挂载到路由器（仅 controller 使用）
    pub mount: Option<MountFn>,
    /// 路由元数据，用于启动时打印路由表
    pub routes: &'static [RouteDescriptor],
}

inventory::collect!(ComponentEntry);
