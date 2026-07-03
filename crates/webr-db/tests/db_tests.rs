//! Integration tests for webr-db config and error types (no database required).

use webr_db::{DatasourceConfig, DbError, PoolConfig};

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
fn resolve_url_returns_explicit_url_when_set() {
    let cfg = DatasourceConfig {
        driver: "sqlite".into(),
        url: Some("sqlite:///tmp/explicit.db".into()),
        host: None,
        port: None,
        username: None,
        password: None,
        database: None,
        pool: PoolConfig::default(),
    };
    assert_eq!(cfg.resolve_url().unwrap(), "sqlite:///tmp/explicit.db");
}

#[test]
fn resolve_url_unsupported_driver_returns_config_error() {
    let cfg = DatasourceConfig {
        driver: "oracle".into(),
        url: None,
        host: None,
        port: None,
        username: None,
        password: None,
        database: None,
        pool: PoolConfig::default(),
    };
    let err = cfg.resolve_url().unwrap_err();
    match err {
        DbError::Config(msg) => assert!(msg.contains("oracle"), "got: {msg}"),
        other => panic!("expected DbError::Config, got: {other:?}"),
    }
}

#[cfg(feature = "sqlite")]
#[test]
fn resolve_url_sqlite_from_fields() {
    let cfg = DatasourceConfig {
        driver: "sqlite".into(),
        url: None,
        host: None,
        port: None,
        username: None,
        password: None,
        database: Some("test.db".into()),
        pool: PoolConfig::default(),
    };
    assert_eq!(cfg.resolve_url().unwrap(), "sqlite://test.db");
}

#[cfg(feature = "postgres")]
#[test]
fn resolve_url_postgres_defaults() {
    let cfg = DatasourceConfig {
        driver: "postgres".into(),
        url: None,
        host: None,
        port: None,
        username: None,
        password: None,
        database: Some("mydb".into()),
        pool: PoolConfig::default(),
    };
    assert_eq!(
        cfg.resolve_url().unwrap(),
        "postgres://postgres:@localhost:5432/mydb"
    );
}

#[cfg(feature = "postgres")]
#[test]
fn resolve_url_postgres_custom_fields() {
    let cfg = DatasourceConfig {
        driver: "postgres".into(),
        url: None,
        host: Some("db.example.com".into()),
        port: Some(5433),
        username: Some("admin".into()),
        password: Some("secret".into()),
        database: Some("prod".into()),
        pool: PoolConfig::default(),
    };
    assert_eq!(
        cfg.resolve_url().unwrap(),
        "postgres://admin:secret@db.example.com:5433/prod"
    );
}

#[cfg(feature = "mysql")]
#[test]
fn resolve_url_mysql_defaults() {
    let cfg = DatasourceConfig {
        driver: "mysql".into(),
        url: None,
        host: None,
        port: None,
        username: None,
        password: None,
        database: Some("mydb".into()),
        pool: PoolConfig::default(),
    };
    assert_eq!(
        cfg.resolve_url().unwrap(),
        "mysql://root:@localhost:3306/mydb"
    );
}

#[cfg(feature = "mysql")]
#[test]
fn resolve_url_mysql_custom_fields() {
    let cfg = DatasourceConfig {
        driver: "mysql".into(),
        url: None,
        host: Some("mysql.local".into()),
        port: Some(3307),
        username: Some("user1".into()),
        password: Some("pass".into()),
        database: Some("app".into()),
        pool: PoolConfig::default(),
    };
    assert_eq!(
        cfg.resolve_url().unwrap(),
        "mysql://user1:pass@mysql.local:3307/app"
    );
}

// ── DbError Display ──────────────────────────────────────────────────

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
