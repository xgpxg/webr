use std::sync::atomic::{AtomicUsize, Ordering};
use webr::prelude::*;

// 自定义组件配置
// 对于需要调用方控制的配置项，放到此处
#[config(prefix = "custom-component")]
pub struct CustomConfig {
    // 这是一个必填配置
    pub prefix: String,
    // 这是一个可选配置，使用 Option
    pub suffix: Option<String>,
}

// 自定义组件
#[component]
pub struct CustomComponent {
    // 注入组件配置
    config: Inject<CustomConfig>,
    // 非 Inject 字段，使用 Default::default() 初始化
    call_count: AtomicUsize,
}

// 实现自定义组件的一些功能
impl CustomComponent {
    pub fn greet(&self, name: &str) -> String {
        self.call_count.fetch_add(1, Ordering::Relaxed);
        let default_suffix = "!".to_string();
        let suffix = self.config.suffix.as_ref().unwrap_or(&default_suffix);
        format!("{}, {}{}", self.config.prefix, name, suffix,)
    }

    pub fn get_call_count(&self) -> usize {
        self.call_count.load(Ordering::Relaxed)
    }
}
