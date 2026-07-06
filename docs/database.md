# 数据库模块

支持 MySQL / PostgreSQL / SQLite 三种数据库的 ORM 和查询功能，通过 Cargo features 开启。

## 启用

```toml
[dependencies]
webr = { version = "0.1", features = ["mysql"] }   # 或 "postgres", "sqlite"
```

## 数据源配置

`config/application.toml`：

```toml
[datasource]
driver = "sqlite"
database = "todos.db"

# 或 PostgreSQL:
# driver = "postgres"
# url = "postgres://user:pass@localhost:5432/mydb"
# host = "localhost"
# port = 5432
# username = "user"
# password = "pass"

[datasource.pool]
max_connections = 10
min_connections = 0
connect_timeout_secs = 30
idle_timeout_secs = 600
```

如果配置了 `url`，则直接使用完整连接字符串；否则根据 `driver` + 各字段拼接。

## 初始化连接池

```rust
use webr::db::{DbPool, DatasourceConfig};

#[webr::main]
async fn main(app: &mut AppBuilder) -> Result<(), Error> {
    let ds_config = app.config()
        .get::<DatasourceConfig>("datasource")
        .map_err(|e| Error::Internal(e.to_string()))?;

    let pool = DbPool::from_config(&ds_config).await
        .map_err(|e| Error::Database(Box::new(e)))?;

    webr::db::set_pool(pool.inner().clone()); // 设置全局池
    app.provide(pool)?;                        // 注册到 DI 容器
    Ok(())
}
```

或者启用 `auto-init` feature 自动初始化：

```toml
webr = { features = ["sqlite", "auto-init"] }
```

`auto-init` 自动检测 `[datasource]` 配置节，自动创建连接池并注册到 DI 容器。

## #[entity] 实体定义

```rust
#[webr::entity(table = "todos")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    #[column(pk)]           // 标记主键
    pub id: i64,
    pub title: String,
    pub done: bool,
}
```

`#[entity]` 宏自动生成：
- `Iden` 枚举（用于 sea-query 构建查询）
- CRUD 方法：`find_all()`, `find_by_id()`, `save()`, `delete()`
- 字段属性：`#[column(pk)]` 主键、`#[column(name = "col")]` 自定义列名

### CRUD 示例

```rust
// 查询全部（自动使用全局池）
let todos = Todo::find_all().await?;

// 按 ID 查询
let todo = Todo::find_by_id(&42).await?;

// 保存（INSERT + 返回完整记录）
let saved = todo.save().await?;

// 删除
let deleted = todo.delete().await?;
```

## #[sql] 动态查询

支持 MyBatis 风格的动态 SQL 标签。

### 基础用法

```rust
impl Todo {
    #[sql(r#"SELECT * FROM todos WHERE id = #{id}"#)]
    pub async fn find_by_id(pool: &webr::db::DbPool, id: i64) -> Result<Option<Self>> {
        unreachable!() // 宏替换为实际实现
    }
}
```

### 动态标签

**`<if>`** — 条件判断：

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

**`<where>`** — 自动处理 WHERE 关键字，去除多余的 AND/OR：

```rust
// 当 title = None, done = Some(true) 时生成：
// SELECT * FROM todos WHERE done = ?
```

**`<foreach>`** — 集合遍历：

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

**`<choose>/<when>/<otherwise>`** — 条件选择：

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

**`<trim>`** — 自定义前缀后缀修整。

### 自定义返回类型

`#[sql]` 支持任意 `sqlx::FromRow` 返回类型和元组：

```rust
// 自定义结构体
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TodoStats {
    pub total: i64,
    pub done_count: i64,
}

#[sql(r#"SELECT COUNT(*) as total, SUM(CASE WHEN done THEN 1 ELSE 0 END) as done_count FROM todos"#)]
pub async fn stats(pool: &webr::db::DbPool) -> Result<TodoStats> {
    unreachable!()
}

// 元组返回
#[sql(r#"SELECT id, title FROM todos"#)]
pub async fn list_tuples(pool: &webr::db::DbPool) -> Result<Vec<(i64, String)>> {
    unreachable!()
}
```

### 分页查询

使用 `Pagination` 参数进行分页：

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
    pager: Pagination,   // 自动识别为分页参数，不参与 SQL 绑定
) -> Result<webr::db::Page<Self>> {
    unreachable!()
}

// 使用
let pager = Pagination::new(1, 20);
let page = Todo::search_page(&pool, Some("rust"), pager).await?;
// page.items, page.total, page.page, page.page_size, page.total_pages, page.has_next, page.has_prev
```

`Page<T>` 字段：

| 字段 | 类型 | 说明 |
|------|------|------|
| items | Vec\<T\> | 当前页数据 |
| total | i64 | 总记录数 |
| page | u64 | 当前页码 |
| page_size | u64 | 每页条数 |
| total_pages | u64 | 总页数 |
| has_next | bool | 是否有下一页 |
| has_prev | bool | 是否有上一页 |

## #[tx] 事务管理

### 声明式事务

在 impl block 上标注 `#[tx]`，其下所有 `async fn` 自动包装在事务中：

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
            todos.push(todo.save().await?); // 使用当前事务
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

事务特性：

- **自动 commit/rollback**：函数返回 `Ok` → commit，返回 `Err` → rollback
- **REQUIRED 传播**：嵌套调用时加入外层事务
- **默认使用 struct 的 `pool` 字段**；可用 `#[tx(pool = "db_pool")]` 覆盖

### 手动事务

```rust
use webr::db::{DbTransaction, scope_txn, try_get_txn};

let txn = DbTransaction::begin(&pool).await?;
let result = scope_txn(&txn, async {
    // 事务中的操作...
    Ok::<_, Error>(())
}).await;
txn.commit().await?; // 或 txn.rollback().await?;
```

## DbPool 直接查询

```rust
// fetch_all: 查询多行
pool.fetch_all::<Todo>("SELECT * FROM todos WHERE done = ?", |b| b.bind(false)).await?;

// fetch_optional: 查询可选单行
pool.fetch_optional::<Todo>("SELECT * FROM todos WHERE id = ?", |b| b.bind(42)).await?;

// fetch_one: 查询确切一行（无数据则报错）
pool.fetch_one::<Todo>("SELECT * FROM todos WHERE id = ?", |b| b.bind(42)).await?;

// execute: INSERT/UPDATE/DELETE，返回影响行数
pool.execute("UPDATE todos SET done = ? WHERE id = ?", |b| b.bind(true).bind(42)).await?;

// fetch_scalar: 标量查询
let count: i64 = pool.fetch_scalar("SELECT COUNT(*) FROM todos", |b| b).await?;
```
