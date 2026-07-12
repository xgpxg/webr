# 文件上传与下载

## 文件上传

使用 `Multipart` 提取器处理 `multipart/form-data` 文件上传：

```rust
use webr::Multipart;

#[controller]
struct FileController;

impl FileController {
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
}
```

## 文件下载

使用 `FileResponse` 返回文件给客户端。

### 从字节构造

```rust
#[controller]
struct FileController;

impl FileController {
    #[get("/download/report")]
    async fn download_report(&self) -> FileResponse {
        let data = vec![/* ... */];
        FileResponse::bytes(data, "report.pdf")
    }
}
```

### 从文件路径读取（缓冲模式）

```rust
#[controller]
struct FileController;

impl FileController {
    #[get("/download/{filename}")]
    async fn download(&self, Path(filename): Path<String>) -> Result<FileResponse, Error> {
        let path = format!("./uploads/{}", filename);
        FileResponse::from_path(&path).await.map_err(|_| Error::Http {
            status: StatusCode::NOT_FOUND,
            message: "File not found".into(),
        })
    }
}
```

### 大文件流式传输

```rust
#[controller]
struct FileController;

impl FileController {
    #[get("/stream/{filename}")]
    async fn stream_file(&self, Path(filename): Path<String>) -> Result<FileResponse, Error> {
        FileResponse::from_path_streaming(format!("./uploads/{}", filename)).await.map_err(|_| Error::Http {
            status: StatusCode::NOT_FOUND,
            message: "File not found".into(),
        })
    }
}
```

### 自定义选项

```rust
// 自定义 Content-Type
FileResponse::bytes(data, "data.bin").content_type("application/octet-stream");

// 内联展示（浏览器直接打开，不触发下载）
FileResponse::bytes(data, "image.png").inline();
```
