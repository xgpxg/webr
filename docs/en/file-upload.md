# File Upload & Download

## File upload

Use the `Multipart` extractor to handle `multipart/form-data` file uploads:

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
            // Save file to disk...
            tokio::fs::write(format!("./uploads/{}", name), &data).await.unwrap();
            filenames.push(name);
        }
        Json(filenames)
    }
}
```

## File download

Use `FileResponse` to return files to the client.

### Construct from bytes

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

### Read from file path

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

### Streaming large files

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

### Custom options

```rust
// Custom Content-Type
FileResponse::bytes(data, "data.bin").content_type("application/octet-stream");

// Inline display (open directly in browser, no download triggered)
FileResponse::bytes(data, "image.png").inline();
```
