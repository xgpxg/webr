# Database

Supports ORM and query functionality for MySQL / PostgreSQL / SQLite databases, built on [sqlx](https://github.com/launchbadge/sqlx).

## Enable

```toml
[dependencies]
webr = { version = "0.1", features = ["mysql"] }   # or "postgres", "sqlite"
```

## Datasource configuration

`config/application.toml`:

```toml
[datasource]
# SQLite
url = "sqlite://todos.db?mode=rwc"

# MySQL
# url = "mysql://localhost:3306/db"
# user = "user"
# password = "password"

# PostgreSQL
# url = "postgres://localhost:5432/db"
# user = "user"
# password = "password"


# Connection pool configuration (optional)
[datasource.pool]
max_connections = 10
min_connections = 0
connect_timeout_secs = 30
idle_timeout_secs = 600
```

## Initializing connection pool

### Manual initialization

```rust
use webr::db::{DbPool, DatasourceConfig};

#[webr::main]
async fn main(app: &mut AppBuilder) -> Result<(), Error> {
    // Get datasource configuration
    let ds_config = app.config()
        .get::<DatasourceConfig>("datasource")
        .map_err(|e| Error::Internal(e.to_string()))?;

    // Create connection pool
    let pool = DbPool::from_config(&ds_config).await
        .map_err(|e| Error::Database(Box::new(e)))?;

    // Set global pool
    webr::db::set_pool(pool.inner().clone());

    // Register to container
    app.provide(pool)?;

    Ok(())
}
```

### Auto initialization

Enable the `auto-init` feature to automatically initialize the connection pool.

```toml
webr = { features = ["sqlite", "auto-init"] }
```

`auto-init` automatically detects the `[datasource]` configuration section, creates the connection pool and registers it to the container.

## `#[entity]` macro

Marks a `struct` as a database entity, automatically generating CRUD and related functions.

```rust
#[webr::entity(table = "todos")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    // Mark primary key
    #[column(pk)]
    pub id: i64,
    pub title: String,
    pub done: bool,
}
```

The `#[entity]` macro automatically generates the following functions:

| Function | Return type | Description |
|----------|-------------|-------------|
| `find_by_id(id: &PkType)` | `Result<Option<Self>>` | Find single record by primary key |
| `find_all()` | `Result<Vec<Self>>` | Find all records |
| `find_page(pager: Pagination)` | `Result<Page<Self>>` | Paginated query |
| `save(&self)` | `Result<()>` | Insert entity, ignoring `None` fields |
| `save_batch(items: &[Self])` | `Result<u64>` | Batch insert, generates single INSERT statement |
| `update(&self)` | `Result<bool>` | Update by primary key, only updates `Some` fields |
| `delete(&self)` | `Result<bool>` | Delete by primary key |
| `count()` | `Result<i64>` | Count total records |

### CRUD example

```rust
// Find all
let todos = Todo::find_all().await?;

// Find by ID
let todo = Todo::find_by_id(42).await?;

// Save (INSERT + returns full record)
let saved = todo.save().await?;

// Delete
let deleted = todo.delete().await?;
```

## #[sql] macro

Supports MyBatis-style dynamic SQL tags.

### Basic usage

```rust
impl Todo {
    #[sql(r#"SELECT * FROM todos WHERE id = #{id}"#)]
    pub async fn find_by_id(pool: &webr::db::DbPool, id: i64) -> Result<Option<Self>> {
        unreachable!() // Macro replaces with actual implementation
    }
}
```

### Dynamic tags

**`<if>`** — Conditional:

```rust
#[sql(r#"
    SELECT * FROM todos
    <where>
        <if test="title">AND title = #{title}</if>
        <if test="done">AND done = #{done}</if>
    </where>
"#)]
pub async fn search(
    pool: &webr::db::DbPool,
    title: Option<&str>,
    done: Option<bool>,
) -> Result<Vec<Self>> {
    unreachable!()
}
```

**`<where>`** — Conditional query:

```rust
// When title = None, done = Some(true), generates:
// SELECT * FROM todos WHERE done = ?
```

**`<foreach>`** — Collection iteration:

```rust
#[sql(r#"
    SELECT * FROM todos WHERE id IN
    <foreach collection="ids" item="id" open="(" separator="," close=")">
        #{id}
    </foreach>
"#)]
pub async fn find_by_ids(
    pool: &webr::db::DbPool,
    ids: &[i64],
) -> Result<Vec<Self>> {
    unreachable!()
}
```

**`<choose>/<when>/<otherwise>`** — Conditional selection:

```rust
#[sql(r#"
    SELECT * FROM todos
    <choose>
        <when test="sort_by">ORDER BY #{sort_by}</when>
        <otherwise>ORDER BY id ASC</otherwise>
    </choose>
"#)]
pub async fn search_sorted(
    pool: &webr::db::DbPool,
    sort_by: Option<&str>,
) -> Result<Vec<Self>> {
    unreachable!()
}
```

**`<trim>`** — Custom prefix/suffix with auto-trimming:

```rust
#[sql(r#"
    UPDATE todos
    <trim prefix="SET" suffixOverrides=",">
        <if test="title">title = #{title},</if>
        <if test="done">done = #{done},</if>
    </trim>
    WHERE id = #{id}
"#)]
pub async fn update_optional(
    pool: &webr::db::DbPool,
    id: i64,
    title: Option<&str>,
    done: Option<bool>,
) -> Result<()> {
    unreachable!()
}
```

### Custom return types

`#[sql]` supports any `sqlx::FromRow` return type and tuples:

```rust
// Custom struct
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TodoStats {
    pub total: i64,
    pub done_count: i64,
}

#[sql(r#"SELECT COUNT(*) as total, SUM(CASE WHEN done THEN 1 ELSE 0 END) as done_count FROM todos"#)]
pub async fn stats(pool: &webr::db::DbPool) -> Result<TodoStats> {
    unreachable!()
}

// Tuple return
#[sql(r#"SELECT id, title FROM todos"#)]
pub async fn list_tuples(pool: &webr::db::DbPool) -> Result<Vec<(i64, String)>> {
    unreachable!()
}
```

### Paginated queries

Use `Pagination` parameter for pagination.

```rust
use webr::db::Pagination;

#[sql(r#"
    SELECT * FROM todos
    <where>
        <if test="title">AND title LIKE #{title}</if>
    </where>
    ORDER BY id
"#)]
pub async fn search_page(
    pool: &webr::db::DbPool,
    title: Option<&str>,
    pager: Pagination,   // Auto-recognized as pagination parameter, not bound to SQL
) -> Result<webr::db::Page<Self>> {
    unreachable!()
}

// Usage
let pager = Pagination::new(1, 20);
let page = Todo::search_page(&pool, Some("rust"), pager).await?;
```

`Page<T>` fields:

| Field | Type | Description |
|-------|------|-------------|
| items | Vec\<T\> | Current page data |
| total | i64 | Total record count |
| page | u64 | Current page number |
| page_size | u64 | Page size |
| total_pages | u64 | Total pages |
| has_next | bool | Has next page |
| has_prev | bool | Has previous page |

## Transactions

### `#[tx]` declarative transactions

Annotate an impl block with `#[tx]`, all `async fn` below it are automatically wrapped in transactions:

```rust
#[tx]
impl TodoService {
    pub async fn create_batch(&self, titles: &[&str]) -> Result<Vec<Todo>> {
        let mut todos = Vec::new();
        for title in titles {
            let todo = Todo {
                id: 0,
                title: title.to_string(),
                done: false,
            };
            todos.push(todo.save().await?); // Uses current transaction
        }
        Ok(todos) // Ok → commit
    }

    pub async fn create_and_fail(&self, title: &str) -> Result<Todo> {
        let todo = Todo { id: 0, title: title.to_string(), done: false };
        let saved = todo.save().await?;
        Err(Error::Internal("rollback".into())) // Err → rollback
    }
}
```

Transaction features:

- **Auto commit/rollback**: Function returns `Ok` → commit, returns `Err` → rollback
- **REQUIRED propagation**: Nested calls join the outer transaction
- **Uses struct's `pool` field by default**; override with `#[tx(pool = "db_pool")]`

### Manual transactions

```rust
use webr::db::{DbTransaction, scope_txn, try_get_txn};

let txn = DbTransaction::begin(&pool).await?;
let result = scope_txn(&txn, async {
    // Operations within transaction...
    Ok::<_, Error>(())
}).await;
txn.commit().await?; // or txn.rollback().await?;
```

## DbPool direct queries

```rust
// fetch_all: query multiple rows
pool.fetch_all::<Todo>("SELECT * FROM todos WHERE done = ?", |b| b.bind(false)).await?;

// fetch_optional: query optional single row
pool.fetch_optional::<Todo>("SELECT * FROM todos WHERE id = ?", |b| b.bind(42)).await?;

// fetch_one: query exactly one row (errors if no data)
pool.fetch_one::<Todo>("SELECT * FROM todos WHERE id = ?", |b| b.bind(42)).await?;

// execute: INSERT/UPDATE/DELETE, returns affected row count
pool.execute("UPDATE todos SET done = ? WHERE id = ?", |b| b.bind(true).bind(42)).await?;

// fetch_scalar: scalar query
let count: i64 = pool.fetch_scalar("SELECT COUNT(*) FROM todos", |b| b).await?;
```
