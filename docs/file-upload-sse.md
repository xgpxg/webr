# 文件下载与 SSE

## 文件上传

使用 `Multipart` 提取器处理 `multipart/form-data` 文件上传：

```rust
use webr::Multipart;

#[post("/upload")]
async fn upload(&self, mut multipart: Multipart) -> Json<Vec<String>> {
    let mut filenames = Vec::new();
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.file_name().unwrap_or("unknown").to_string();
        let data = field.bytes().await.unwrap();
        // 保存文件到磁盘...
        tokio::fs::write(format!("./uploads/{}", name), &data).await.unwrap();
        filenames.push(name);
    }
    Json(filenames)
}
```

## 文件下载

使用 `FileResponse` 返回文件给客户端。

### 从字节构造

```rust
#[get("/download/report")]
async fn download_report(&self) -> FileResponse {
    let data = vec![/* ... */];
    FileResponse::bytes(data, "report.pdf")
}
```

### 从文件路径读取（缓冲模式）

```rust
#[get("/download/{filename}")]
async fn download(&self, Path(filename): Path<String>) -> Result<FileResponse, Error> {
    let path = format!("./uploads/{}", filename);
    FileResponse::from_path(&path).await.map_err(|_| Error::Http {
        status: StatusCode::NOT_FOUND,
        message: "File not found".into(),
    })
}
```

### 大文件流式传输

```rust
#[get("/stream/{filename}")]
async fn stream_file(&self, Path(filename): Path<String>) -> Result<FileResponse, Error> {
    FileResponse::from_path_streaming(format!("./uploads/{}", filename)).await.map_err(|_| Error::Http {
        status: StatusCode::NOT_FOUND,
        message: "File not found".into(),
    })
}
```

### 自定义选项

```rust
// 自定义 Content-Type
FileResponse::bytes(data, "data.bin").content_type("application/octet-stream");

// 内联展示（浏览器直接打开，不触发下载）
FileResponse::bytes(data, "image.png").inline();
```

FileResponse 自动：
- 根据文件扩展名推断 Content-Type（支持 pdf、图片、音视频、文档等常见格式）
- 构造合适的 `Content-Disposition` 头
- 非 ASCII 文件名自动进行 RFC 5987 编码

## SSE (Server-Sent Events)

使用 `SseResponse` 和 `SseEvent` 实现服务端推送。

### 基础用法

```rust
use webr::response::{SseResponse, SseEvent};
use futures_util::stream;
use std::time::Duration;

#[get("/events")]
async fn stream(&self) -> SseResponse {
    let events = stream::iter(vec![
        SseEvent::new("hello"),
        SseEvent::new("world").event("greeting"),
    ]);
    SseResponse::new(events)
        .keep_alive(Duration::from_secs(15)) // 每 15 秒发送心跳，防止连接超时
}
```

### 动态生成事件流

```rust
use webr::response::{SseResponse, SseEvent};
use tokio::sync::mpsc;

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
```

### SseEvent 选项

```rust
SseEvent::new("data content")           // 仅数据
    .event("chat")                       // 事件类型 (event:)
    .id("msg-1")                         // 事件 ID (id:)
    .retry(Duration::from_secs(3))       // 客户端重连间隔 (retry:)
    .comment("this is a comment")        // 注释行（客户端忽略）
```

### SseResponse 选项

```rust
SseResponse::new(stream)
    .keep_alive(Duration::from_secs(15)) // 心跳间隔，防止代理/负载均衡断开连接
```
