//! Integration tests for DbPool, DbTransaction, and query executors.
//! Uses SQLite in-memory database — no external service required.

#![cfg(feature = "sqlite")]
use webr_db::{
    scope_txn, try_get_txn, DatasourceConfig, DbPool, DbTransaction, Driver, PoolConfig,
};

/// Test entity mapping to a `todos` table.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
struct Todo {
    id: i64,
    title: String,
    done: bool,
}

/// Build a SQLite in-memory pool.
async fn test_pool() -> DbPool {
    let cfg = DatasourceConfig {
        driver: "sqlite".into(),
        url: Some("sqlite::memory:".into()),
        host: None,
        port: None,
        username: None,
        password: None,
        database: None,
        pool: PoolConfig::default(),
    };
    DbPool::from_config(&cfg).await.expect("failed to create pool")
}

/// Initialise the `todos` table inside a fresh pool.
async fn setup() -> DbPool {
    let pool = test_pool().await;
    pool.execute(
        "CREATE TABLE todos (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL, done INTEGER NOT NULL DEFAULT 0)",
        |b| b,
    )
    .await
    .expect("CREATE TABLE failed");
    pool
}

// ── Pool basics ──────────────────────────────────────────────────────

#[tokio::test]
async fn pool_driver_is_sqlite() {
    let pool = test_pool().await;
    assert_eq!(pool.driver(), Driver::Sqlite);
}

#[tokio::test]
async fn pool_placeholder_returns_question_mark() {
    let pool = test_pool().await;
    assert_eq!(pool.placeholder(1), "?");
    assert_eq!(pool.placeholder(2), "?");
}

#[tokio::test]
async fn pool_debug_format() {
    let pool = test_pool().await;
    let dbg = format!("{pool:?}");
    assert!(dbg.contains("DbPool"), "got: {dbg}");
    assert!(dbg.contains("Sqlite"), "got: {dbg}");
}

// ── execute / fetch_all / fetch_one / fetch_optional ──────────────────

#[tokio::test]
async fn execute_returns_affected_row_count() {
    let pool = setup().await;
    let affected = pool
        .execute("INSERT INTO todos (title, done) VALUES (?, ?)", |b| {
            b.bind("buy milk").bind(0i64)
        })
        .await
        .unwrap();
    assert_eq!(affected, 1);
}

#[tokio::test]
async fn fetch_all_returns_all_rows() {
    let pool = setup().await;
    pool.execute("INSERT INTO todos (title, done) VALUES (?, ?)", |b| {
        b.bind("task a").bind(0i64)
    })
    .await
    .unwrap();
    pool.execute("INSERT INTO todos (title, done) VALUES (?, ?)", |b| {
        b.bind("task b").bind(1i64)
    })
    .await
    .unwrap();

    let rows: Vec<Todo> = pool
        .fetch_all("SELECT id, title, done FROM todos ORDER BY id", |b| b)
        .await
        .unwrap();

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].title, "task a");
    assert_eq!(rows[1].title, "task b");
}

#[tokio::test]
async fn fetch_one_returns_single_row() {
    let pool = setup().await;
    pool.execute("INSERT INTO todos (title, done) VALUES (?, ?)", |b| {
        b.bind("only one").bind(0i64)
    })
    .await
    .unwrap();

    let row: Todo = pool
        .fetch_one("SELECT id, title, done FROM todos WHERE id = ?", |b| b.bind(1i64))
        .await
        .unwrap();

    assert_eq!(row.title, "only one");
}

#[tokio::test]
async fn fetch_optional_returns_none_when_no_match() {
    let pool = setup().await;
    let row: Option<Todo> = pool
        .fetch_optional("SELECT id, title, done FROM todos WHERE id = ?", |b| b.bind(999i64))
        .await
        .unwrap();
    assert!(row.is_none());
}

#[tokio::test]
async fn fetch_optional_returns_some_when_match() {
    let pool = setup().await;
    pool.execute("INSERT INTO todos (title, done) VALUES (?, ?)", |b| {
        b.bind("exists").bind(0i64)
    })
    .await
    .unwrap();

    let row: Option<Todo> = pool
        .fetch_optional("SELECT id, title, done FROM todos WHERE id = ?", |b| b.bind(1i64))
        .await
        .unwrap();
    assert!(row.is_some());
    assert_eq!(row.unwrap().title, "exists");
}

// ── fetch_scalar ─────────────────────────────────────────────────────

#[tokio::test]
async fn fetch_scalar_count() {
    let pool = setup().await;
    pool.execute("INSERT INTO todos (title, done) VALUES (?, ?)", |b| {
        b.bind("a").bind(0i64)
    })
    .await
    .unwrap();
    pool.execute("INSERT INTO todos (title, done) VALUES (?, ?)", |b| {
        b.bind("b").bind(0i64)
    })
    .await
    .unwrap();

    let count: i64 = pool
        .fetch_scalar("SELECT COUNT(*) FROM todos", |b| b)
        .await
        .unwrap();
    assert_eq!(count, 2);
}

// ── insert_fetch ─────────────────────────────────────────────────────

#[tokio::test]
async fn insert_fetch_returns_created_entity() {
    let pool = setup().await;
    let todo: Todo = pool
        .insert_fetch(
            "INSERT INTO todos (title, done) VALUES (?, ?)",
            "SELECT id, title, done FROM todos WHERE id = ?",
            "id",
            |b| b.bind("new todo").bind(0i64),
        )
        .await
        .unwrap();

    assert_eq!(todo.title, "new todo");
    assert!(!todo.done);
    assert!(todo.id > 0);
}

// ── Transaction: commit ──────────────────────────────────────────────

#[tokio::test]
async fn transaction_commit_persists_data() {
    let pool = setup().await;

    let txn = DbTransaction::begin(&pool).await.unwrap();
    txn.execute("INSERT INTO todos (title, done) VALUES (?, ?)", |b| {
        b.bind("committed").bind(0i64)
    })
    .await
    .unwrap();
    txn.commit().await.unwrap();

    let count: i64 = pool
        .fetch_scalar("SELECT COUNT(*) FROM todos", |b| b)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

// ── Transaction: rollback ────────────────────────────────────────────

#[tokio::test]
async fn transaction_rollback_discards_data() {
    let pool = setup().await;

    let txn = DbTransaction::begin(&pool).await.unwrap();
    txn.execute("INSERT INTO todos (title, done) VALUES (?, ?)", |b| {
        b.bind("discarded").bind(0i64)
    })
    .await
    .unwrap();
    txn.rollback().await.unwrap();

    let count: i64 = pool
        .fetch_scalar("SELECT COUNT(*) FROM todos", |b| b)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

// ── Transaction: fetch_all inside txn ────────────────────────────────

#[tokio::test]
async fn transaction_fetch_all_sees_uncommitted_rows() {
    let pool = setup().await;
    let txn = DbTransaction::begin(&pool).await.unwrap();

    txn.execute("INSERT INTO todos (title, done) VALUES (?, ?)", |b| {
        b.bind("visible").bind(0i64)
    })
    .await
    .unwrap();

    let rows: Vec<Todo> = txn
        .fetch_all("SELECT id, title, done FROM todos", |b| b)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].title, "visible");

    txn.rollback().await.unwrap();
}

// ── Transaction: insert_fetch inside txn ─────────────────────────────

#[tokio::test]
async fn transaction_insert_fetch() {
    let pool = setup().await;
    let txn = DbTransaction::begin(&pool).await.unwrap();

    let todo: Todo = txn
        .insert_fetch(
            "INSERT INTO todos (title, done) VALUES (?, ?)",
            "SELECT id, title, done FROM todos WHERE id = ?",
            "id",
            |b| b.bind("txn todo").bind(0i64),
        )
        .await
        .unwrap();

    assert_eq!(todo.title, "txn todo");
    assert!(todo.id > 0);

    txn.commit().await.unwrap();

    // Verify it persisted
    let row: Todo = pool
        .fetch_one("SELECT id, title, done FROM todos WHERE id = ?", |b| b.bind(todo.id))
        .await
        .unwrap();
    assert_eq!(row.title, "txn todo");
}

// ── Transaction: fetch_scalar inside txn ─────────────────────────────

#[tokio::test]
async fn transaction_fetch_scalar() {
    let pool = setup().await;
    let txn = DbTransaction::begin(&pool).await.unwrap();

    txn.execute("INSERT INTO todos (title, done) VALUES (?, ?)", |b| {
        b.bind("x").bind(0i64)
    })
    .await
    .unwrap();
    txn.execute("INSERT INTO todos (title, done) VALUES (?, ?)", |b| {
        b.bind("y").bind(0i64)
    })
    .await
    .unwrap();

    let count: i64 = txn
        .fetch_scalar("SELECT COUNT(*) FROM todos", |b| b)
        .await
        .unwrap();
    assert_eq!(count, 2);

    txn.rollback().await.unwrap();
}

// ── scope_txn + try_get_txn ──────────────────────────────────────────

#[tokio::test]
async fn try_get_txn_returns_none_outside_scope() {
    assert!(try_get_txn().is_none());
}

#[tokio::test]
async fn scope_txn_exposes_transaction_via_try_get_txn() {
    let pool = setup().await;
    let txn = DbTransaction::begin(&pool).await.unwrap();

    let result = scope_txn(&txn, async {
        let inner = try_get_txn();
        assert!(inner.is_some(), "try_get_txn should return Some inside scope_txn");
        inner.unwrap().driver()
    })
    .await;

    assert_eq!(result, Driver::Sqlite);
    txn.rollback().await.unwrap();
}

// ── Unsupported driver error ─────────────────────────────────────────

#[tokio::test]
async fn from_config_unsupported_driver_returns_error() {
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
    let result = DbPool::from_config(&cfg).await;
    assert!(result.is_err());
}
