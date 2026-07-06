use std::any::TypeId;

use crate::context::FactoryFn;
use std::any::Any;

/// 所有托管组件必须实现的 trait，由 #[controller] / #[service] 宏自动 derive
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
    /// 依赖的 TypeId 列表，用于拓扑排序
    pub dependencies: Vec<TypeId>,
    /// 工厂函数：从 ApplicationContext 创建组件实例
    pub factory: FactoryFn<E>,
}
