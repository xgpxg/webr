# SSE (Server-Sent Events)

Use `SseResponse` and `SseEvent` to implement server-side push.

## Basic usage

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
            .keep_alive(Duration::from_secs(15)) // Send heartbeat every 15s to prevent connection timeout
    }
}

```

## Dynamically generated event streams

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
