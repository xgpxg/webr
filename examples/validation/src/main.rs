use serde::{Deserialize, Serialize};
use webr::prelude::*;
use webr::Error;

// ─── 参数校验 DTO ──────────────────────────────────────

/// 创建用户请求体
#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserDto {
    #[validate(length(min = 1, max = 50, message = "name length must be between 1 and 50"))]
    pub name: String,
    
    #[validate(email(message = "invalid email format"))]
    pub email: String,
    
    #[validate(range(min = 18, max = 150, message = "age must be between 18 and 150"))]
    pub age: u8,
}

/// 登录表单
#[derive(Debug, Deserialize, Validate)]
pub struct LoginForm {
    #[validate(length(min = 1, message = "username is required"))]
    pub username: String,
    
    #[validate(length(min = 6, message = "password must be at least 6 characters"))]
    pub password: String,
}

/// 搜索查询参数（Query 参数校验）
#[derive(Debug, Deserialize, Validate)]
pub struct SearchQuery {
    #[validate(length(min = 1, max = 100, message = "keyword length must be between 1 and 100"))]
    pub keyword: Option<String>,
    
    #[validate(range(min = 1, message = "page must be at least 1"))]
    pub page: Option<u32>,
    
    #[validate(range(min = 1, max = 100, message = "page_size must be between 1 and 100"))]
    pub page_size: Option<u32>,
}

// ─── 响应类型 ──────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub age: u8,
}

// ─── Controller ─────────────────────────────────────────

#[controller]
pub struct ValidationController;

#[controller]
impl ValidationController {
    #[get("/")]
    async fn index(&self) -> &'static str {
        "Validation Example - Try POST /users, POST /login, or GET /search?keyword=..."
    }

    /// 创建用户：JSON Body 校验
    /// 校验失败自动返回 400 + 详细错误信息
    #[post("/users")]
    async fn create_user(
        &self, 
        webr::Json(dto): webr::Json<CreateUserDto>
    ) -> webr::Json<UserResponse> {
        // 如果代码执行到这里，说明校验已通过
        webr::Json(UserResponse {
            id: 1,
            name: dto.name,
            email: dto.email,
            age: dto.age,
        })
    }

    /// 登录：Form 表单校验
    #[post("/login")]
    async fn login(
        &self, 
        webr::Form(form): webr::Form<LoginForm>
    ) -> webr::Json<serde_json::Value> {
        webr::Json(serde_json::json!({
            "token": "mock-jwt-token",
            "username": form.username
        }))
    }

    /// 搜索：Query 参数校验
    #[get("/search")]
    async fn search(
        &self, 
        webr::Query(query): webr::Query<SearchQuery>
    ) -> webr::Json<serde_json::Value> {
        webr::Json(serde_json::json!({
            "keyword": query.keyword.unwrap_or_default(),
            "page": query.page.unwrap_or(1),
            "page_size": query.page_size.unwrap_or(10),
            "results": []
        }))
    }
}

// ─── 启动入口 ──────────────────────────────────────────

#[webr::main]
async fn main(app: &mut webr::AppBuilder) -> Result<(), Error> {
    // 统一响应格式
    app.unified_response();
    Ok(())
}

// ─── 单元测试 ──────────────────────────────────────────

#[cfg(test)]
mod tests {
    use webr::validator::Validate;
    use super::*;

    // ─── CreateUserDto ─────────────────────────────────

    #[test]
    fn create_user_valid() {
        let dto = CreateUserDto {
            name: "Alice".into(),
            email: "alice@example.com".into(),
            age: 25,
        };
        assert!(dto.validate().is_ok());
    }

    #[test]
    fn create_user_empty_name() {
        let dto = CreateUserDto {
            name: "".into(),
            email: "alice@example.com".into(),
            age: 25,
        };
        let err = dto.validate().unwrap_err();
        assert!(err.field_errors().contains_key("name"));
    }

    #[test]
    fn create_user_name_too_long() {
        let dto = CreateUserDto {
            name: "a".repeat(51),
            email: "alice@example.com".into(),
            age: 25,
        };
        let err = dto.validate().unwrap_err();
        assert!(err.field_errors().contains_key("name"));
    }

    #[test]
    fn create_user_invalid_email() {
        let dto = CreateUserDto {
            name: "Alice".into(),
            email: "not-an-email".into(),
            age: 25,
        };
        let err = dto.validate().unwrap_err();
        assert!(err.field_errors().contains_key("email"));
    }

    #[test]
    fn create_user_age_too_young() {
        let dto = CreateUserDto {
            name: "Alice".into(),
            email: "alice@example.com".into(),
            age: 10,
        };
        let err = dto.validate().unwrap_err();
        assert!(err.field_errors().contains_key("age"));
    }

    #[test]
    fn create_user_age_boundary() {
        // min boundary (18)
        let dto = CreateUserDto {
            name: "Alice".into(),
            email: "alice@example.com".into(),
            age: 18,
        };
        assert!(dto.validate().is_ok());

        // max boundary (150)
        let dto = CreateUserDto {
            name: "Alice".into(),
            email: "alice@example.com".into(),
            age: 150,
        };
        assert!(dto.validate().is_ok());
    }

    // ─── LoginForm ─────────────────────────────────────

    #[test]
    fn login_valid() {
        let form = LoginForm {
            username: "alice".into(),
            password: "123456".into(),
        };
        assert!(form.validate().is_ok());
    }

    #[test]
    fn login_empty_username() {
        let form = LoginForm {
            username: "".into(),
            password: "123456".into(),
        };
        let err = form.validate().unwrap_err();
        assert!(err.field_errors().contains_key("username"));
    }

    #[test]
    fn login_short_password() {
        let form = LoginForm {
            username: "alice".into(),
            password: "12345".into(),
        };
        let err = form.validate().unwrap_err();
        assert!(err.field_errors().contains_key("password"));
    }

    // ─── SearchQuery ───────────────────────────────────

    #[test]
    fn search_all_none_valid() {
        let q = SearchQuery {
            keyword: None,
            page: None,
            page_size: None,
        };
        assert!(q.validate().is_ok());
    }

    #[test]
    fn search_valid_values() {
        let q = SearchQuery {
            keyword: Some("rust".into()),
            page: Some(1),
            page_size: Some(20),
        };
        assert!(q.validate().is_ok());
    }

    #[test]
    fn search_empty_keyword() {
        let q = SearchQuery {
            keyword: Some("".into()),
            page: Some(1),
            page_size: Some(10),
        };
        let err = q.validate().unwrap_err();
        assert!(err.field_errors().contains_key("keyword"));
    }

    #[test]
    fn search_page_zero() {
        let q = SearchQuery {
            keyword: None,
            page: Some(0),
            page_size: None,
        };
        let err = q.validate().unwrap_err();
        assert!(err.field_errors().contains_key("page"));
    }

    #[test]
    fn search_page_size_over_limit() {
        let q = SearchQuery {
            keyword: None,
            page: None,
            page_size: Some(101),
        };
        let err = q.validate().unwrap_err();
        assert!(err.field_errors().contains_key("page_size"));
    }
}
