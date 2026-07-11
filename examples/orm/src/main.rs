use serde::{Deserialize, Serialize};
use webr::prelude::*;
use webr::{Error, Inject};

// ─── ORM 实体：Todo ─────────────────────────────────────

#[webr::entity(table = "todos")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    #[column(pk)]
    pub id: Option<i64>,
    pub title: Option<String>,
    pub done: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTodoDto {
    pub title: String,
}

// ─── 复杂 #[sql] 查询：动态标签演示 ──────────────────────

/// 搜索查询参数
#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub title: Option<String>,
    pub done: Option<bool>,
    pub sort_by: Option<String>,
    pub ids: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

// ─── 自定义返回类型：聚合统计 ────────────────────────

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TodoStats {
    pub total: i64,
    pub done_count: i64,
}

impl Todo {
    /// fetch_one：自定义返回类型，聚合统计
    #[sql(r#"SELECT COUNT(*) FROM todos"#)]
    pub async fn count_all(pool: &webr::db::DbPool) -> webr::Result<u64> {
        unreachable!()
    }
    /// fetch_one：自定义返回类型，聚合统计
    #[sql(r#"SELECT COUNT(*) as total, SUM(CASE WHEN done THEN 1 ELSE 0 END) as done_count FROM todos"#)]
    pub async fn stats(pool: &webr::db::DbPool) -> webr::Result<TodoStats> {
        unreachable!()
    }

    /// fetch_optional + 自定义返回类型
    #[sql(
        r#"SELECT COUNT(*) as total, SUM(CASE WHEN done THEN 1 ELSE 0 END) as done_count FROM todos
        <where>
            <if test="title">AND title LIKE #{title}</if>
        </where>"#
    )]
    pub async fn stats_by_title(
        pool: &webr::db::DbPool,
        title: Option<&str>,
    ) -> webr::Result<Option<TodoStats>> {
        unreachable!()
    }

    // ─── 元组返回类型示例 ──────────────────────────────────

    /// 静态 SQL：fetch_one 返回元组 (id, title)
    #[sql(r#"SELECT id, title FROM todos ORDER BY id LIMIT 1"#)]
    pub async fn first_tuple(pool: &webr::db::DbPool) -> webr::Result<(i64, String)> {
        unreachable!()
    }

    /// 静态 SQL + 参数：按 title 查找，返回元组 (id, title)
    #[sql(r#"SELECT id, title FROM todos WHERE title = #{title}"#)]
    pub async fn find_tuple_by_title(
        pool: &webr::db::DbPool,
        title: &str,
    ) -> webr::Result<Option<(i64, String)>> {
        unreachable!()
    }

    /// 静态 SQL：fetch_all 返回元组列表 (id, title, done)
    #[sql(r#"SELECT id, title, done FROM todos"#)]
    pub async fn list_tuples(
        pool: &webr::db::DbPool,
    ) -> webr::Result<Vec<(i64, String, bool)>> {
        unreachable!()
    }

    /// 动态 SQL + 参数：按 title 可选过滤，返回元组列表 (id, title)
    #[sql(
        r#"SELECT id, title FROM todos
        <where>
            <if test="title">AND title = #{title}</if>
        </where>"#
    )]
    pub async fn search_tuples(
        pool: &webr::db::DbPool,
        title: Option<&str>,
    ) -> webr::Result<Vec<(i64, String)>> {
        unreachable!()
    }

    // ─── 分页查询示例 ──────────────────────────────────────

    /// 动态 SQL 分页：带条件过滤 + 分页
    #[sql(
        r#"SELECT * FROM todos
        <where>
            <if test="title">AND title LIKE #{title}</if>
            <if test="done">AND done = #{done}</if>
        </where>
        ORDER BY id"#
    )]
    pub async fn search_page(
        pool: &webr::db::DbPool,
        title: Option<&str>,
        done: Option<bool>,
        pager: webr::db::Pagination,
    ) -> webr::Result<webr::db::Page<Self>> {
        unreachable!()
    }
}

impl Todo {
    /// 动态 <where> + <if>：可选过滤条件
    #[sql(
        r#"SELECT * FROM todos
        <where>
            <if test="title">AND title = #{title}</if>
            <if test="done">AND done = #{done}</if>
        </where>"#
    )]
    pub async fn search(
        pool: &webr::db::DbPool,
        title: Option<&str>,
        done: Option<bool>,
    ) -> webr::Result<Vec<Self>> {
        unreachable!()
    }

    /// <foreach>：批量 ID 查询
    #[sql(
        r#"SELECT * FROM todos WHERE id IN
        <foreach collection="ids" item="id" open="(" separator="," close=")">
            #{id}
        </foreach>"#
    )]
    pub async fn find_by_ids(
        pool: &webr::db::DbPool,
        ids: &[i64],
    ) -> webr::Result<Vec<Self>> {
        unreachable!()
    }

    /// <choose>/<when>/<otherwise>：条件排序
    #[sql(
        r#"SELECT * FROM todos
        <where>
            <if test="title">AND title LIKE #{title}</if>
        </where>
        <choose>
            <when test="sort_by">ORDER BY #{sort_by}</when>
            <otherwise>ORDER BY id ASC</otherwise>
        </choose>"#
    )]
    pub async fn search_sorted(
        pool: &webr::db::DbPool,
        title: Option<&str>,
        sort_by: Option<&str>,
    ) -> webr::Result<Vec<Self>> {
        unreachable!()
    }
}

// ─── Service 层 ─────────────────────────────────────────

#[component]
pub struct TodoService {
    pool: Inject<webr::db::DbPool>,
}

impl TodoService {
    pub async fn list(&self) -> webr::Result<Vec<Todo>> {
        Ok(Todo::find_all().await?)
    }

    pub async fn get(&self, id: i64) -> webr::Result<Option<Todo>> {
        let one = Todo::find_by_id(&Some(id)).await?;
        Ok(one)
    }

    pub async fn create(&self, title: &str) -> webr::Result<Todo> {
        let todo = Todo {
            id: None,
            title: Some(title.to_string()),
            done: Some(false),
        };
        todo.save().await?;
        Ok(todo)
    }

    pub async fn delete(&self, id: i64) -> webr::Result<bool> {
        if let Some(todo) = Todo::find_by_id(&Some(id)).await? {
            Ok(todo.delete().await?)
        } else {
            Ok(false)
        }
    }

    pub async fn search(
        &self,
        title: Option<&str>,
        done: Option<bool>,
    ) -> webr::Result<Vec<Todo>> {
        Ok(Todo::search(&self.pool, title, done).await?)
    }

    pub async fn find_by_ids(&self, ids: &[i64]) -> webr::Result<Vec<Todo>> {
        Ok(Todo::find_by_ids(&self.pool, ids).await?)
    }

    pub async fn search_sorted(
        &self,
        title: Option<&str>,
        sort_by: Option<&str>,
    ) -> webr::Result<Vec<Todo>> {
        Ok(Todo::search_sorted(&self.pool, title, sort_by).await?)
    }

    pub async fn count(&self) -> webr::Result<i64> {
        Ok(Todo::count().await?)
    }

    pub async fn count_all(&self) -> webr::Result<u64> {
        Ok(Todo::count_all(&self.pool).await?)
    }

    pub async fn stats(&self) -> webr::Result<TodoStats> {
        Ok(Todo::stats(&self.pool).await?)
    }

    pub async fn stats_by_title(
        &self,
        title: Option<&str>,
    ) -> webr::Result<Option<TodoStats>> {
        Ok(Todo::stats_by_title(&self.pool, title).await?)
    }

    pub async fn first_tuple(&self) -> webr::Result<(i64, String)> {
        Ok(Todo::first_tuple(&self.pool).await?)
    }

    pub async fn find_tuple_by_title(
        &self,
        title: &str,
    ) -> webr::Result<Option<(i64, String)>> {
        Ok(Todo::find_tuple_by_title(&self.pool, title).await?)
    }

    pub async fn list_tuples(&self) -> webr::Result<Vec<(i64, String, bool)>> {
        Ok(Todo::list_tuples(&self.pool).await?)
    }

    pub async fn search_tuples(
        &self,
        title: Option<&str>,
    ) -> webr::Result<Vec<(i64, String)>> {
        Ok(Todo::search_tuples(&self.pool, title).await?)
    }

    /// 分页查询
    pub async fn find_page(
        &self,
        pager: webr::db::Pagination,
    ) -> webr::Result<webr::db::Page<Todo>> {
        Ok(Todo::find_page(pager).await?)
    }

    /// 带条件的分页查询
    pub async fn search_page(
        &self,
        title: Option<&str>,
        done: Option<bool>,
        pager: webr::db::Pagination,
    ) -> webr::Result<webr::db::Page<Todo>> {
        Ok(Todo::search_page(&self.pool, title, done, pager).await?)
    }

    /// 事务提交测试：批量创建多个 todo，全部成功后提交
    #[tx]
    pub async fn create_batch(&self, titles: &[&str]) -> webr::Result<Vec<Todo>> {
        let mut todos = Vec::new();
        for title in titles {
            let todo = Todo {
                id: None,
                title: Some(title.to_string()),
                done: Some(false),
            };
            todo.save().await?;
            todos.push(todo);
        }
        Ok(todos)
    }

    /// 事务回滚测试：创建一条后故意失败，验证全部回滚
    #[tx]
    pub async fn create_and_fail(&self, title: &str) -> webr::Result<Todo> {
        let todo = Todo {
            id: None,
            title: Some(title.to_string()),
            done: Some(false),
        };
        todo.save().await?;
        // 故意返回错误 → 触发 rollback
        Err(Error::Internal("intentional rollback".into()))
    }
}

// ─── Controller 层 ──────────────────────────────────────

#[controller]
pub struct TodoController {
    todo_service: Inject<TodoService>,
}

#[controller(prefix = "/api")]
impl TodoController {
    #[get("/todos")]
    async fn list_todos(&self) -> webr::Result<webr::Json<Vec<Todo>>> {
        Ok(webr::Json(self.todo_service.list().await?))
    }

    #[get("/todos/{id}")]
    async fn get_todo(
        &self,
        webr::Path(id): webr::Path<i64>,
    ) -> webr::Result<webr::Json<Todo>> {
        match self.todo_service.get(id).await? {
            Some(todo) => Ok(webr::Json(todo)),
            None => Err(Error::Http {
                status: StatusCode::NOT_FOUND,
                message: format!("Todo {id} not found"),
            }),
        }
    }

    #[post("/todos")]
    async fn create_todo(
        &self,
        webr::Json(dto): webr::Json<CreateTodoDto>,
    ) -> webr::Result<webr::Json<Todo>> {
        Ok(webr::Json(self.todo_service.create(&dto.title).await?))
    }

    #[delete("/todos/{id}")]
    async fn delete_todo(
        &self,
        webr::Path(id): webr::Path<i64>,
    ) -> webr::Result<webr::Json<serde_json::Value>> {
        let deleted = self.todo_service.delete(id).await?;
        Ok(webr::Json(serde_json::json!({"deleted": deleted})))
    }

    /// GET /api/todos/search?title=...&done=...
    #[get("/todos/search")]
    async fn search_todos(
        &self,
        webr::Query(params): webr::Query<SearchParams>,
    ) -> webr::Result<webr::Json<Vec<Todo>>> {
        let title = params.title.as_deref();
        let done = params.done;
        Ok(webr::Json(self.todo_service.search(title, done).await?))
    }

    /// GET /api/todos/by-ids?ids=1,2,3
    #[get("/todos/by-ids")]
    async fn find_todos_by_ids(
        &self,
        webr::Query(params): webr::Query<SearchParams>,
    ) -> webr::Result<webr::Json<Vec<Todo>>> {
        let ids: Vec<i64> = params
            .ids
            .as_deref()
            .unwrap_or("")
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        Ok(webr::Json(self.todo_service.find_by_ids(&ids).await?))
    }

    /// GET /api/todos/sorted?title=...&sort_by=...
    #[get("/todos/sorted")]
    async fn search_sorted_todos(
        &self,
        webr::Query(params): webr::Query<SearchParams>,
    ) -> webr::Result<webr::Json<Vec<Todo>>> {
        let title = params.title.as_deref();
        let sort_by = params.sort_by.as_deref();
        Ok(webr::Json(self.todo_service.search_sorted(title, sort_by).await?))
    }

    /// GET /api/todos/count-all — #[sql] COUNT(*) 查询
    #[get("/todos/count-all")]
    async fn count_all_todos(&self) -> webr::Result<webr::Json<serde_json::Value>> {
        let count = self.todo_service.count_all().await?;
        Ok(webr::Json(serde_json::json!({"count": count})))
    }

    /// GET /api/todos/stats — 聚合统计（自定义返回类型）
    #[get("/todos/stats")]
    async fn stats_todos(&self) -> webr::Result<webr::Json<TodoStats>> {
        Ok(webr::Json(self.todo_service.stats().await?))
    }

    /// GET /api/todos/stats/by-title?title=... — 按标题过滤的聚合统计
    #[get("/todos/stats/by-title")]
    async fn stats_by_title(
        &self,
        webr::Query(params): webr::Query<SearchParams>,
    ) -> webr::Result<webr::Json<serde_json::Value>> {
        let title = params.title.as_deref();
        match self.todo_service.stats_by_title(title).await? {
            Some(stats) => Ok(webr::Json(serde_json::json!(stats))),
            None => Ok(webr::Json(serde_json::json!({"total": 0, "done_count": 0}))),
        }
    }

    // ─── 事务测试端点 ────────────────────────────────────

    /// GET /api/todos/tuple/first — 静态 SQL 返回元组 (id, title)
    #[get("/todos/tuple/first")]
    async fn first_tuple_todo(&self) -> webr::Result<webr::Json<serde_json::Value>> {
        let (id, title) = self.todo_service.first_tuple().await?;
        Ok(webr::Json(serde_json::json!({"id": id, "title": title})))
    }

    /// GET /api/todos/tuple/by-title?title=... — 静态 SQL + 参数返回元组
    #[get("/todos/tuple/by-title")]
    async fn tuple_by_title(
        &self,
        webr::Query(params): webr::Query<SearchParams>,
    ) -> webr::Result<webr::Json<serde_json::Value>> {
        let title = params.title.as_deref().unwrap_or("");
        match self.todo_service.find_tuple_by_title(title).await? {
            Some((id, t)) => Ok(webr::Json(serde_json::json!({"id": id, "title": t}))),
            None => Ok(webr::Json(serde_json::json!({"found": false}))),
        }
    }

    /// GET /api/todos/tuple/list — 静态 SQL 返回元组列表 (id, title, done)
    #[get("/todos/tuple/list")]
    async fn list_tuple_todos(&self) -> webr::Result<webr::Json<serde_json::Value>> {
        let tuples = self.todo_service.list_tuples().await?;
        Ok(webr::Json(serde_json::json!({
            "items": tuples.iter().map(|(id, title, done)| {
                serde_json::json!({"id": id, "title": title, "done": done})
            }).collect::<Vec<_>>()
        })))
    }

    /// GET /api/todos/tuple/search?title=... — 动态 SQL + 参数返回元组列表
    #[get("/todos/tuple/search")]
    async fn search_tuple_todos(
        &self,
        webr::Query(params): webr::Query<SearchParams>,
    ) -> webr::Result<webr::Json<serde_json::Value>> {
        let title = params.title.as_deref();
        let tuples = self.todo_service.search_tuples(title).await?;
        Ok(webr::Json(serde_json::json!({
            "items": tuples.iter().map(|(id, title)| {
                println!("tuple: {:?}", (id, title));
                serde_json::json!({"id": id, "title": title})
            }).collect::<Vec<_>>()
        })))
    }

    /// POST /api/todos/tx/commit — 批量创建 3 条 todo，验证事务提交
    #[post("/todos/tx/commit")]
    async fn tx_commit(&self) -> webr::Result<webr::Json<serde_json::Value>> {
        let todos = self
            .todo_service
            .create_batch(&["tx-a", "tx-b", "tx-c"])
            .await?;
        Ok(webr::Json(serde_json::json!({
            "committed": true,
            "count": todos.len(),
            "titles": todos.iter().map(|t| t.title.as_deref().unwrap_or("")).collect::<Vec<_>>(),
        })))
    }

    /// POST /api/todos/tx/rollback — 创建 1 条后故意失败，验证回滚
    #[post("/todos/tx/rollback")]
    async fn tx_rollback(&self) -> webr::Result<webr::Json<serde_json::Value>> {
        let before = self.todo_service.count().await?;
        let result = self.todo_service.create_and_fail("should-not-exist").await;
        let after = self.todo_service.count().await?;
        Ok(webr::Json(serde_json::json!({
            "rolled_back": result.is_err(),
            "count_before": before,
            "count_after": after,
            "no_leak": before == after,
        })))
    }

    // ─── 分页查询端点 ────────────────────────────────────────

    /// GET /api/todos/page?page=1&page_size=10 — 分页查询
    #[get("/todos/page")]
    async fn page_todos(
        &self,
        webr::Query(params): webr::Query<SearchParams>,
    ) -> webr::Result<webr::Json<webr::db::Page<Todo>>> {
        let page = params.page.unwrap_or(1);
        let page_size = params.page_size.unwrap_or(10);
        let pager = webr::db::Pagination::new(page, page_size);
        let p = self.todo_service.find_page(pager).await?;
        Ok(webr::Json(p))
    }

    /// GET /api/todos/page/search?title=...&done=...&page=1&page_size=10 — 条件分页
    #[get("/todos/page/search")]
    async fn search_page_todos(
        &self,
        webr::Query(params): webr::Query<SearchParams>,
    ) -> webr::Result<webr::Json<webr::db::Page<Todo>>> {
        let title = params.title.as_deref();
        let done = params.done;
        let page = params.page.unwrap_or(1);
        let page_size = params.page_size.unwrap_or(10);
        let pager = webr::db::Pagination::new(page, page_size);
        let p = self.todo_service.search_page(title, done, pager).await?;
        Ok(webr::Json(p))
    }
}

// ─── 启动入口 ──────────────────────────────────────────

#[webr::main]
async fn main(app: &mut webr::AppBuilder) -> webr::Result<()> {
    app.middleware(webr::LoggerMiddleware);
    // 初始化 SQLite 数据库
    let ds_config = app
        .config()
        .get::<webr::db::DatasourceConfig>("datasource")
        .map_err(|e| Error::Internal(e.to_string()))?;
    let pool = webr::db::DbPool::from_config(&ds_config).await.map_err(|e| Error::Database(Box::new(e)))?;

    // 创建 todos 表
    webr::db::sqlx::query(
        "CREATE TABLE IF NOT EXISTS todos (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL, done BOOLEAN NOT NULL DEFAULT 0)"
    )
    .execute(pool.as_sq())
    .await
    .map_err(|e| Error::Database(Box::new(webr::db::DbError::Sqlx(e))))?;

    // Store pool globally for #[entity] generated methods
    webr::db::set_pool(pool.inner().clone());
    // 提供连接池到 DI 容器
    app.provide(pool)?;
    // 统一响应格式
    app.unified_response();
    Ok(())
}
