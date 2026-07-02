use std::ops::Deref;
use std::sync::Arc;

use crate::component::Component;

/// 依赖注入智能指针
pub struct Inject<T: Component> {
    /// 组件实例的引用计数指针
    inner: Arc<T>,
}

impl<T: Component> Inject<T> {
    /// 构造注入指针，仅由容器装配阶段调用
    pub fn new(inner: Arc<T>) -> Self {
        Self { inner }
    }

    /// 克隆内部 `Arc`，用于需要拥有独立引用计数的场景
    pub fn arc(&self) -> Arc<T> {
        Arc::clone(&self.inner)
    }
}

/// 透明代理，允许 `Inject<T>` 直接调用 `T` 的方法
impl<T: Component> Deref for Inject<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// 浅克隆，多个 `Inject` 共享同一组件实例
impl<T: Component> Clone for Inject<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// 调试输出格式：`Inject<ComponentName>`
impl<T: Component + std::fmt::Debug> std::fmt::Debug for Inject<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Inject<{}>", T::component_name())
    }
}
