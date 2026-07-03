use serde::Deserialize;

use crate::error::Error;

/// 服务器配置，对应 `[server]` 配置节
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    /// 监听端口，默认 8080
    #[serde(default = "default_port")]
    pub port: u16,
    /// 监听地址，默认 "0.0.0.0"
    #[serde(default = "default_host")]
    pub host: String,
    /// 请求体最大字节数，默认 2MB
    #[serde(default = "default_max_body_size")]
    pub max_body_size: usize,
}

fn default_port() -> u16 {
    8080
}

fn default_host() -> String {
    "0.0.0.0".into()
}

/// 默认请求体上限：2MB
fn default_max_body_size() -> usize {
    2 * 1024 * 1024
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            host: default_host(),
            max_body_size: default_max_body_size(),
        }
    }
}

/// 日志配置，对应 `[log]` 配置节
#[derive(Debug, Clone, Deserialize)]
pub struct LogConfig {
    /// 日志级别，默认 "info"
    #[serde(default = "default_log_level")]
    pub level: String,
}

fn default_log_level() -> String {
    "info".into()
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

/// 配置条目，由 `#[config]` 宏通过 `inventory::submit!` 提交，
/// 启动时由 `AppBuilder::build()` 收集并注册到 IoC 容器。
pub struct ConfigEntry {
    /// 从 toml 根节点解析配置类型并注册到 IoC 容器
    pub register: fn(&toml::Value, &mut crate::context::ApplicationContext) -> Result<(), Error>,
}

inventory::collect!(ConfigEntry);

/// 配置加载器，支持多文件合并与环境变量覆盖。
///
/// 优先级（后者覆盖前者）：
/// 1. 内置默认值
/// 2. `config/application.toml`
/// 3. `config/application-{profile}.toml`
/// 4. 环境变量（`WEBR_` 前缀，如 `WEBR_SERVER_PORT=9090`）
pub struct ConfigLoader {
    /// 合并后的配置值
    values: toml::Value,
    /// 当前激活的 profile
    profile: String,
    /// 已加载的配置文件路径
    files_loaded: Vec<String>,
}

impl ConfigLoader {
    /// 按优先级加载配置源，返回 `ConfigLoader` 实例
    pub fn load() -> Result<Self, Error> {
        // 加载 .env 文件（忽略不存在的错误）
        let _ = dotenvy::dotenv();

        // 确定 profile
        let profile = std::env::var("WEBR_PROFILE").unwrap_or_else(|_| "dev".into());

        let mut values = toml::Value::Table(toml::Table::new());
        let mut files_loaded = Vec::new();

        // 1. config/application.toml
        let base_path = "config/application.toml";
        if let Some(base) = read_toml_file(base_path)? {
            merge_toml(&mut values, base);
            files_loaded.push(base_path.to_string());
        }

        // 2. config/application-{profile}.toml
        let profile_path = format!("config/application-{profile}.toml");
        if let Some(profile_val) = read_toml_file(&profile_path)? {
            merge_toml(&mut values, profile_val);
            files_loaded.push(profile_path);
        }

        // 3. 环境变量覆盖（WEBR_ 前缀）
        for (key, val) in std::env::vars() {
            if let Some(config_key) = key.strip_prefix("WEBR_") {
                if config_key == "PROFILE" {
                    continue;
                }
                let toml_key = config_key.to_lowercase().replace("__", ".");
                set_env_override(&mut values, &toml_key, &val);
            }
        }

        Ok(Self {
            values,
            profile,
            files_loaded,
        })
    }

    /// 当前激活的 profile，默认 "dev"
    pub fn profile(&self) -> &str {
        &self.profile
    }

    /// 已加载的配置文件路径列表
    pub fn files_loaded(&self) -> &[String] {
        &self.files_loaded
    }

    /// 将指定配置节反序列化为类型 `T`
    pub fn get<T: for<'de> Deserialize<'de>>(&self, section: &str) -> Result<T, Error> {
        let val = self
            .values
            .get(section)
            .cloned()
            .unwrap_or_else(|| toml::Value::Table(toml::Table::new()));
        T::deserialize(val)
            .map_err(|e| Error::ConfigError(format!("Failed to parse [{section}]: {e}")))
    }

    /// 返回原始 toml 值，供 `#[config]` 宏生成的代码使用
    pub fn raw(&self) -> &toml::Value {
        &self.values
    }

    /// 解析 `[server]` 配置节为 `ServerConfig`
    pub(crate) fn server_config(&self) -> ServerConfig {
        self.get::<ServerConfig>("server").unwrap_or_default()
    }
}

/// 读取 TOML 文件，文件不存在返回 `None`
fn read_toml_file(path: &str) -> Result<Option<toml::Value>, Error> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(None);
        }
        Err(e) => {
            return Err(Error::ConfigError(format!("Cannot read {path}: {e}")));
        }
    };
    let val = content
        .parse::<toml::Value>()
        .map_err(|e| Error::ConfigError(format!("Invalid TOML in {path}: {e}")))?;
    Ok(Some(val))
}

/// 深度合并 toml 值，`source` 中的同名键覆盖 `target`
fn merge_toml(target: &mut toml::Value, source: toml::Value) {
    match (target, source) {
        (toml::Value::Table(ref mut t), toml::Value::Table(s)) => {
            for (k, v) in s {
                let entry = t.entry(k).or_insert(toml::Value::Table(toml::Table::new()));
                merge_toml(entry, v);
            }
        }
        (target, source) => *target = source,
    }
}

/// 将环境变量写入配置树，`key` 以点号分隔层级（如 `"server.port"`）
fn set_env_override(values: &mut toml::Value, key: &str, val: &str) {
    let parts: Vec<&str> = key.split('.').collect();

    // 确保根节点为 Table
    if !values.is_table() {
        *values = toml::Value::Table(toml::Table::new());
    }

    // 沿路径导航到叶子节点的父级
    let mut current = values;
    for &part in &parts[..parts.len() - 1] {
        let table = current
            .as_table_mut()
            .expect("parent was ensured to be table");
        let next = table
            .entry(part)
            .or_insert_with(|| toml::Value::Table(toml::Table::new()));
        // 中间节点若非 Table 则重置
        if !next.is_table() {
            *next = toml::Value::Table(toml::Table::new());
        }
        current = next;
    }

    // 写入叶子节点（自动推断类型）
    let leaf = parts.last().expect("key must have at least one part");
    if let Some(table) = current.as_table_mut() {
        table.insert(leaf.to_string(), parse_env_value(val));
    }
}

/// 将字符串解析为 `toml::Value`，按 i64 → f64 → bool → string 顺序尝试
fn parse_env_value(val: &str) -> toml::Value {
    if let Ok(i) = val.parse::<i64>() {
        return toml::Value::Integer(i);
    }
    if let Ok(f) = val.parse::<f64>() {
        return toml::Value::Float(f);
    }
    match val {
        "true" => toml::Value::Boolean(true),
        "false" => toml::Value::Boolean(false),
        _ => toml::Value::String(val.to_string()),
    }
}
