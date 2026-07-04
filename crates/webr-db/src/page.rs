//! 分页支持类型。

use serde::Serialize;

/// 分页参数。
///
/// 在 `#[sql]` 宏方法中作为参数使用时，宏会自动识别并提取分页信息，
/// 不参与 SQL 参数绑定。
///
/// # Example
/// ```ignore
/// #[sql("SELECT * FROM todos")]
/// pub async fn find_page(pool: &DbPool, pager: Pagination) -> db::Result<Page<Todo>> {
///     unreachable!()
/// }
///
/// let page = Todo::find_page(&pool, Pagination::new(1, 20)).await?;
/// ```
#[derive(Debug, Clone)]
pub struct Pagination {
    /// 页码（从 1 开始）
    pub page: u64,
    /// 每页条数
    pub page_size: u64,
}

impl Pagination {
    /// 创建分页请求。
    ///
    /// `page` 最小为 1，`page_size` 最小为 1。
    pub fn new(page: u64, page_size: u64) -> Self {
        Self {
            page: page.max(1),
            page_size: page_size.max(1),
        }
    }

    /// 计算偏移量（用于 SQL OFFSET）。
    #[inline]
    pub fn offset(&self) -> i64 {
        ((self.page.saturating_sub(1)) * self.page_size) as i64
    }

    /// 获取 LIMIT 值。
    #[inline]
    pub fn limit(&self) -> i64 {
        self.page_size as i64
    }
}

/// 分页结果。
///
/// 包含当前页数据、总记录数、页码等分页元信息。
#[derive(Debug, Serialize)]
pub struct Page<T> {
    /// 当前页数据
    pub items: Vec<T>,
    /// 总记录数
    pub total: i64,
    /// 当前页码
    pub page: u64,
    /// 每页条数
    pub page_size: u64,
    /// 总页数
    pub total_pages: u64,
    /// 是否有下一页
    pub has_next: bool,
    /// 是否有上一页
    pub has_prev: bool,
}

impl<T> Page<T> {
    /// 创建分页结果。
    pub fn new(items: Vec<T>, total: i64, page: u64, page_size: u64) -> Self {
        let total_pages = if total <= 0 || page_size == 0 {
            0
        } else {
            ((total as u64) + page_size - 1) / page_size
        };
        let has_next = page < total_pages;
        let has_prev = page > 1;
        Self {
            items,
            total,
            page,
            page_size,
            total_pages,
            has_next,
            has_prev,
        }
    }

    /// 是否为空页。
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 映射分页数据，保持分页元信息不变。
    pub fn map<U>(self, f: impl FnMut(T) -> U) -> Page<U> {
        Page {
            items: self.items.into_iter().map(f).collect(),
            total: self.total,
            page: self.page,
            page_size: self.page_size,
            total_pages: self.total_pages,
            has_next: self.has_next,
            has_prev: self.has_prev,
        }
    }
}
