use serde::Deserialize;

/// Database connection pool configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct DatasourceConfig {
    /// Driver: "mysql", "postgres", or "sqlite"
    pub driver: String,
    /// Full connection URL, e.g. "postgres://host:5432/mydb"
    pub url: String,
    /// Optional username injected into the URL (replaces any embedded credentials)
    pub username: Option<String>,
    /// Optional password injected into the URL
    pub password: Option<String>,
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
    /// Resolve the final connection URL.
    ///
    /// Merges `username`/`password` into the URL's authority section when configured.
    pub fn resolve_url(&self) -> Result<String, crate::DbError> {
        Ok(self.merge_credentials(&self.url))
    }

    /// Inject `username`/`password` into an existing URL, replacing any embedded credentials.
    /// Returns the URL unchanged if neither field is configured.
    fn merge_credentials(&self, url: &str) -> String {
        let user = self.username.as_deref();
        let pass = self.password.as_deref();
        if user.is_none() && pass.is_none() {
            return url.to_string();
        }
        let Some((scheme, rest)) = url.split_once("://") else {
            return url.to_string();
        };
        // Strip existing credentials (everything before '@' in the authority)
        let rest = rest.split_once('@').map_or(rest, |(_, after)| after);
        // Build credential prefix
        let creds = match (user, pass) {
            (Some(u), Some(p)) => format!("{u}:{p}@"),
            (Some(u), None) => format!("{u}@"),
            (None, Some(p)) => format!(":{p}@"),
            _ => String::new(),
        };
        format!("{scheme}://{creds}{rest}")
    }
}
