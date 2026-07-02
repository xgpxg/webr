use serde::{Deserialize, Serialize};
use webr::prelude::*;
use webr::{Inject, WebrError};

#[config(prefix = "app")]
pub struct AppConfig {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub greeting: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: i64,
    pub name: String,
}

#[controller]
pub struct HelloController {
    app_config: Inject<AppConfig>,
}

#[controller]
impl HelloController {
    #[get("/")]
    async fn index(&self) -> String {
        self.app_config.greeting.clone()
    }

    #[get("/health")]
    async fn health(&self) -> StatusCode {
        StatusCode::OK
    }

    #[get("/info")]
    async fn info(&self) -> String {
        format!("{} v{}", self.app_config.name, self.app_config.version)
    }

    #[get("/panic")]
    async fn trigger_panic(&self) -> &'static str {
        panic!("Something went terribly wrong!");
    }

    // ─── CRUD test endpoints ───

    #[get("/items")]
    async fn list_items(&self) -> Json<Vec<Item>> {
        Json(vec![
            Item {
                id: 1,
                name: "Alice".into(),
            },
            Item {
                id: 2,
                name: "Bob".into(),
            },
        ])
    }

    #[get("/items/{id}")]
    async fn get_item(&self, Path(id): Path<i64>) -> WebrResult<Json<Item>> {
        if id > 0 && id <= 2 {
            Ok(Json(Item {
                id,
                name: format!("Item-{id}"),
            }))
        } else {
            Err(WebrError::Http {
                status: StatusCode::NOT_FOUND,
                message: format!("Item {id} not found"),
            })
        }
    }

    #[post("/items")]
    async fn create_item(&self, Json(body): Json<CreateItemRequest>) -> Json<Item> {
        Json(Item {
            id: 42,
            name: body.name,
        })
    }

    #[put("/items/{id}")]
    async fn update_item(
        &self,
        Path(id): Path<i64>,
        Json(body): Json<CreateItemRequest>,
    ) -> Json<Item> {
        Json(Item {
            id,
            name: body.name,
        })
    }

    #[delete("/items/{id}")]
    async fn delete_item(&self, Path(id): Path<i64>) -> StatusCode {
        let _ = id;
        StatusCode::NO_CONTENT
    }
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateItemRequest {
    pub name: String,
}

#[main]
async fn main(app: &mut AppBuilder) -> Result<(), WebrError> {
    app.unified_response();
    Ok(())
}
