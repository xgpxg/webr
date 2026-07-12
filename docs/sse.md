# SSE (Server-Sent Events)

使用 `SseResponse` 和 `SseEvent` 实现服务端推送。

## 基础用法

```rust
use webr::response::{SseResponse, SseEvent};
use futures_util::stream;
use std::time::Duration;

#[controller]
struct SseController;

impl SseController{
    #[get("/events")]
    async fn stream(&self) -> SseResponse {
        let events = stream::iter(vec![
            SseEvent::new("hello"),
            SseEvent::new("world").event("greeting"),
        ]);
        SseResponse::new(events)
            .keep_alive(Duration::from_secs(15)) // 每 15 秒发送心跳，防止连接超时
    }
}

```

## 动态生成事件流

```rust
use webr::response::{SseResponse, SseEvent};
use tokio::sync::mpsc;

#[controller]
struct SseController;

impl SseController {
    #[get("/events")]
    async fn stream(&self) -> SseResponse {
        SseResponse::new(async_stream::stream! {
            for i in 0..10 {
                let event = SseEvent::new(format!("message {}", i))
                    .event("chat")
                    .id(i.to_string());
                yield Ok(event);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        })
    }
}
```
