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
}

// 实现自定义组件的一些功能
impl CustomComponent {
    pub fn greet(&self, name: &str) -> String {
        format!(
            "{}, {}{}",
            self.config.prefix,
            name,
            self.config.suffix.as_ref().unwrap_or(&"!".to_string()),
        )
    }
}
