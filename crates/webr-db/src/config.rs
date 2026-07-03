use serde::Deserialize;

/// Database connection pool configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct DatasourceConfig {
    /// Driver: "mysql", "postgres", or "sqlite"
    pub driver: String,
    /// Full connection URL (takes precedence over individual fields)
    pub url: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub database: Option<String>,
    /// Pool tuning parameters
    #[serde(default)]
    pub pool: PoolConfig,
}

/// Connection pool tuning.
#[derive(Debug, Clone, Deserialize)]
pub struct PoolConfig {
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    #[serde(default)]
    pub min_connections: u32,
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout_secs: u64,
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_secs: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: default_max_connections(),
            min_connections: 0,
            connect_timeout_secs: default_connect_timeout(),
            idle_timeout_secs: default_idle_timeout(),
        }
    }
}

fn default_max_connections() -> u32 {
    10
}
fn default_connect_timeout() -> u64 {
    30
}
fn default_idle_timeout() -> u64 {
    600
}

impl DatasourceConfig {
    /// Build the connection URL from individual fields if `url` is not set.
    #[allow(unused_variables)]
    pub fn resolve_url(&self) -> Result<String, crate::DbError> {
        if let Some(ref url) = self.url {
            return Ok(url.clone());
        }
        let host = self.host.as_deref().unwrap_or("localhost");
        let database = self.database.as_deref().unwrap_or("");
        match self.driver.as_str() {
            #[cfg(feature = "postgres")]
            "postgres" => {
                let port = self.port.unwrap_or(5432);
                let user = self.username.as_deref().unwrap_or("postgres");
                let pass = self.password.as_deref().unwrap_or("");
                Ok(format!("postgres://{user}:{pass}@{host}:{port}/{database}"))
            }
            #[cfg(feature = "mysql")]
            "mysql" => {
                let port = self.port.unwrap_or(3306);
                let user = self.username.as_deref().unwrap_or("root");
                let pass = self.password.as_deref().unwrap_or("");
                Ok(format!("mysql://{user}:{pass}@{host}:{port}/{database}"))
            }
            #[cfg(feature = "sqlite")]
            "sqlite" => Ok(format!("sqlite://{database}")),
            other => Err(crate::DbError::Config(format!(
                "unsupported driver '{other}'"
            ))),
        }
    }
}
