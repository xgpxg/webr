//! Integration tests for webr-db config and error types (no database required).

use webr_db::{DatasourceConfig, PoolConfig};

// ── PoolConfig defaults ──────────────────────────────────────────────

#[test]
fn pool_config_default_values() {
    let cfg = PoolConfig::default();
    assert_eq!(cfg.max_connections, 10);
    assert_eq!(cfg.min_connections, 0);
    assert_eq!(cfg.connect_timeout_secs, 30);
    assert_eq!(cfg.idle_timeout_secs, 600);
}

// ── DatasourceConfig::resolve_url ────────────────────────────────────

#[test]
fn resolve_url_returns_url_unchanged_when_no_credentials() {
    let cfg = DatasourceConfig {
        driver: "sqlite".into(),
        url: "sqlite:///tmp/explicit.db".into(),
        username: None,
        password: None,
        pool: PoolConfig::default(),
    };
    assert_eq!(cfg.resolve_url().unwrap(), "sqlite:///tmp/explicit.db");
}

#[test]
fn resolve_url_merges_username_and_password() {
    let cfg = DatasourceConfig {
        driver: "postgres".into(),
        url: "postgres://host:5432/mydb".into(),
        username: Some("admin".into()),
        password: Some("secret".into()),
        pool: PoolConfig::default(),
    };
    assert_eq!(
        cfg.resolve_url().unwrap(),
        "postgres://admin:secret@host:5432/mydb"
    );
}

#[test]
fn resolve_url_merges_username_only() {
    let cfg = DatasourceConfig {
        driver: "postgres".into(),
        url: "postgres://host:5432/mydb".into(),
        username: Some("admin".into()),
        password: None,
        pool: PoolConfig::default(),
    };
    assert_eq!(
        cfg.resolve_url().unwrap(),
        "postgres://admin@host:5432/mydb"
    );
}

#[test]
fn resolve_url_replaces_existing_credentials() {
    let cfg = DatasourceConfig {
        driver: "postgres".into(),
        url: "postgres://old:oldpass@host:5432/mydb?sslmode=require".into(),
        username: Some("new".into()),
        password: Some("newpass".into()),
        pool: PoolConfig::default(),
    };
    assert_eq!(
        cfg.resolve_url().unwrap(),
        "postgres://new:newpass@host:5432/mydb?sslmode=require"
    );
}

#[test]
fn resolve_url_returns_url_without_scheme_unchanged() {
    let cfg = DatasourceConfig {
        driver: "sqlite".into(),
        url: "just-a-path.db".into(),
        username: Some("user".into()),
        password: Some("pass".into()),
        pool: PoolConfig::default(),
    };
    // URL 无 scheme 时原样返回，无法注入凭据
    assert_eq!(cfg.resolve_url().unwrap(), "just-a-path.db");
}

// ── DbError Display ──────────────────────────────────────────────────

use webr_db::DbError;

#[test]
fn db_error_display_pool_closed() {
    let err = DbError::PoolClosed;
    assert_eq!(format!("{err}"), "Connection pool is closed");
}

#[test]
fn db_error_display_insert_failed() {
    let err = DbError::InsertFailed;
    assert_eq!(format!("{err}"), "Insert failed: no record returned");
}

#[test]
fn db_error_display_config() {
    let err = DbError::Config("bad url".into());
    let msg = format!("{err}");
    assert!(msg.contains("bad url"), "got: {msg}");
}

#[test]
fn db_error_from_sqlx() {
    // Use a closed-pool error to produce a sqlx::Error variant
    let sqlx_err = sqlx::Error::PoolClosed;
    let db_err: DbError = sqlx_err.into();
    let msg = format!("{db_err}");
    assert!(msg.contains("SQLx error"), "got: {msg}");
}

#[test]
fn db_error_implements_std_error() {
    // Compile-time check: DbError implements std::error::Error
    fn assert_error<E: std::error::Error>() {}
    assert_error::<DbError>();
}
