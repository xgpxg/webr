use std::convert::Infallible;
use std::path::{Path, PathBuf};
use std::time::Duration;

use axum::body::Body;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use futures_util::stream::BoxStream;
use futures_util::{Stream, StreamExt};
use tokio_util::io::ReaderStream;

/// 文件下载响应。
///
/// 支持字节、文件路径两种构造方式，自动推断 `Content-Type`，
/// 默认 `Content-Disposition: attachment`（触发浏览器下载）。
///
/// 大文件可使用 [`from_path_streaming`](Self::from_path_streaming) 流式返回，
/// 避免一次性加载到内存。
///
/// # 示例
///
/// ```rust
/// // 从字节构造
/// FileResponse::bytes(vec, "report.pdf")
///
/// // 从文件路径构造
/// FileResponse::from_path("/data/report.pdf").await?
///
/// // 大文件流式传输
/// FileResponse::from_path_streaming("/data/large.zip").await?
///
/// // 自定义 Content-Type
/// FileResponse::bytes(vec, "data.bin").content_type("application/octet-stream")
///
/// // 内联展示（浏览器直接打开，不下载）
/// FileResponse::bytes(vec, "image.png").inline()
/// ```
pub struct FileResponse {
    /// 响应体（缓冲或流式）
    body: FileBody,
    /// 文件名（用于 Content-Disposition）
    filename: Option<String>,
    /// MIME 类型
    content_type: &'static str,
    /// true = inline（浏览器直接展示），false = attachment（触发下载）
    inline: bool,
}

/// 响应体：缓冲字节或流式文件
enum FileBody {
    /// 已读入内存的字节
    Buffered(Vec<u8>),
    /// 流式文件句柄（大文件场景）
    Streaming(tokio::fs::File),
}

impl FileResponse {
    /// 从字节构造，默认 attachment 模式。
    pub fn bytes(data: Vec<u8>, filename: impl Into<String>) -> Self {
        let filename = filename.into();
        let content_type = guess_mime(&filename).unwrap_or(DEFAULT_MIME);
        Self {
            body: FileBody::Buffered(data),
            filename: Some(filename),
            content_type,
            inline: false,
        }
    }

    /// 从文件路径读取，自动推断 MIME 类型。
    ///
    /// 文件不存在时返回 404。整个文件读入内存。
    pub async fn from_path(path: impl Into<PathBuf>) -> Result<Self, crate::error::WebrError> {
        let path: PathBuf = path.into();
        let data = tokio::fs::read(&path)
            .await
            .map_err(|_| crate::error::WebrError::Http {
                status: StatusCode::NOT_FOUND,
                message: "File not found".into(),
            })?;
        let filename = path.file_name().map(|n| n.to_string_lossy().into_owned());
        let content_type = path
            .file_name()
            .and_then(|n| guess_mime(&n.to_string_lossy()))
            .unwrap_or(DEFAULT_MIME);
        Ok(Self {
            body: FileBody::Buffered(data),
            filename,
            content_type,
            inline: false,
        })
    }

    /// 从文件路径流式读取，适合大文件场景。
    ///
    /// 不会一次性将整个文件加载到内存，而是分块传输。
    /// 文件不存在时返回 404。
    pub async fn from_path_streaming(
        path: impl Into<PathBuf>,
    ) -> Result<Self, crate::error::WebrError> {
        let path: PathBuf = path.into();
        let file =
            tokio::fs::File::open(&path)
                .await
                .map_err(|_| crate::error::WebrError::Http {
                    status: StatusCode::NOT_FOUND,
                    message: "File not found".into(),
                })?;
        let filename = path.file_name().map(|n| n.to_string_lossy().into_owned());
        let content_type = path
            .file_name()
            .and_then(|n| guess_mime(&n.to_string_lossy()))
            .unwrap_or(DEFAULT_MIME);
        Ok(Self {
            body: FileBody::Streaming(file),
            filename,
            content_type,
            inline: false,
        })
    }

    /// 覆盖 Content-Type。
    pub fn content_type(mut self, ct: &'static str) -> Self {
        self.content_type = ct;
        self
    }

    /// 切换为 inline 模式（浏览器直接展示，不触发下载）。
    pub fn inline(mut self) -> Self {
        self.inline = true;
        self
    }
}

impl IntoResponse for FileResponse {
    fn into_response(self) -> axum::response::Response {
        let mut headers = HeaderMap::new();

        // Content-Disposition（含 RFC 5987 编码，支持非 ASCII 文件名）
        if let Some(disp) = build_disposition(self.filename.as_deref(), self.inline) {
            headers.insert(header::CONTENT_DISPOSITION, disp);
        }

        // Content-Type（构造期已保证为合法 ASCII，unwrap 安全）
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static(self.content_type),
        );

        let body = match self.body {
            FileBody::Buffered(data) => Body::from(data),
            FileBody::Streaming(file) => Body::from_stream(ReaderStream::new(file)),
        };

        (StatusCode::OK, headers, body).into_response()
    }
}

// ─── 内部辅助 ──────────────────────────────────────────────

/// 无法推断 MIME 类型时的默认值
const DEFAULT_MIME: &str = "application/octet-stream";

/// 构建 Content-Disposition HeaderValue。
///
/// ASCII 文件名使用 `filename="..."`，非 ASCII 额外附加 RFC 5987 `filename*=UTF-8''...`。
/// 文件名含非法字符时 graceful 降级为无文件名。
fn build_disposition(filename: Option<&str>, inline: bool) -> Option<HeaderValue> {
    let disposition_type = if inline { "inline" } else { "attachment" };

    let value = match filename {
        Some(name) if name.is_ascii() && !name.contains('"') => {
            format!(r#"{disposition_type}; filename="{name}""#)
        }
        Some(name) => {
            let encoded = utf8_percent_encode(name).to_string();
            format!("{disposition_type}; filename*=UTF-8''{encoded}")
        }
        None => disposition_type.to_string(),
    };

    HeaderValue::from_str(&value).ok()
}

/// 最小化 percent-encoding（仅编码 Content-Disposition 中不允许的字符）
fn utf8_percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            // RFC 5987 attr-char: 字母、数字及 !#$&+-.^_`|~
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'!'
            | b'#'
            | b'$'
            | b'&'
            | b'+'
            | b'-'
            | b'.'
            | b'^'
            | b'_'
            | b'`'
            | b'|'
            | b'~' => out.push(byte as char),
            // 空格用 %20
            _ => {
                out.push('%');
                out.push_str(&format!("{byte:02X}"));
            }
        }
    }
    out
}

// ─── MIME 类型推断 ─────────────────────────────────────────

/// 根据文件扩展名推断 MIME 类型，覆盖常见场景。
///
/// 返回 `&'static str` 避免堆分配。
fn guess_mime(filename: &str) -> Option<&'static str> {
    let ext = Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())?;

    let mime = match ext.as_str() {
        // 文档
        "pdf" => "application/pdf",
        "doc" | "docx" => "application/msword",
        "xls" | "xlsx" => "application/vnd.ms-excel",
        "ppt" | "pptx" => "application/vnd.ms-powerpoint",
        "zip" | "gz" | "tar" | "rar" | "7z" => "application/octet-stream",
        "json" => "application/json",
        "xml" => "application/xml",
        // 图片
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        // 音视频
        "mp3" => "audio/mpeg",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        // 文本
        "txt" => "text/plain",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "text/javascript",
        "csv" => "text/csv",
        // 其他
        _ => return None,
    };

    Some(mime)
}

// ─── SSE (Server-Sent Events) ──────────────────────────────

/// 单条 SSE 事件。
///
/// 通过链式调用设置 `event`、`id`、`retry` 等字段。
///
/// # 示例
///
/// ```rust
/// // 仅数据
/// SseEvent::new("hello")
///
/// // 带事件类型和 ID
/// SseEvent::new(r#"{"msg":"hi"}"#).event("chat").id("1")
/// ```
pub struct SseEvent {
    data: String,
    event: Option<String>,
    id: Option<String>,
    retry: Option<Duration>,
    comment: Option<String>,
}

impl SseEvent {
    /// 创建一条仅包含数据的事件。
    pub fn new(data: impl Into<String>) -> Self {
        Self {
            data: data.into(),
            event: None,
            id: None,
            retry: None,
            comment: None,
        }
    }

    /// 设置事件类型（`event:` 字段）。
    pub fn event(mut self, event: impl Into<String>) -> Self {
        self.event = Some(event.into());
        self
    }

    /// 设置事件 ID（`id:` 字段）。
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// 设置客户端重连间隔（`retry:` 字段）。
    pub fn retry(mut self, duration: Duration) -> Self {
        self.retry = Some(duration);
        self
    }

    /// 添加注释行（`:` 前缀，客户端会忽略）。
    pub fn comment(mut self, text: impl Into<String>) -> Self {
        self.comment = Some(text.into());
        self
    }

    /// 转换为 axum Event。
    fn into_axum_event(self) -> axum::response::sse::Event {
        let mut event = axum::response::sse::Event::default().data(self.data);
        if let Some(name) = self.event {
            event = event.event(name);
        }
        if let Some(id) = self.id {
            event = event.id(id);
        }
        if let Some(d) = self.retry {
            event = event.retry(d);
        }
        if let Some(text) = self.comment {
            event = event.comment(text);
        }
        event
    }
}

/// SSE 流式响应。
///
/// 接受 `Stream<Item = SseEvent>` 或 `Stream<Item = Result<SseEvent, E>>` 输入流，
/// 自动设置 `Content-Type: text/event-stream` 并实现 `IntoResponse`。
///
/// # 示例
///
/// ```rust
/// use webr::response::{SseEvent, SseResponse};
/// use futures_util::stream;
/// use std::time::Duration;
///
/// async fn sse_handler() -> SseResponse {
///     let events = stream::iter(vec![
///         SseEvent::new("hello"),
///         SseEvent::new("world").event("greeting"),
///     ]);
///     SseResponse::new(events)
///         .keep_alive(Duration::from_secs(15))
/// }
/// ```
pub struct SseResponse {
    stream: BoxStream<'static, Result<axum::response::sse::Event, Infallible>>,
    keep_alive: Option<Duration>,
}

impl SseResponse {
    /// 从事件流创建 SSE 响应。
    ///
    /// 接受 `Stream<Item = SseEvent>` 或 `Stream<Item = Result<SseEvent, E>>`。
    /// 流中的错误事件会被跳过并记录日志。
    pub fn new<S, E>(stream: S) -> Self
    where
        S: Stream + Send + 'static,
        S::Item: IntoSseEventResult<E> + Send,
        E: std::fmt::Display,
    {
        let boxed = stream.filter_map(|item| async move {
            match item.into_result() {
                Ok(event) => Some(Ok(event.into_axum_event())),
                Err(e) => {
                    tracing::warn!("SSE stream error, skipping event: {e}");
                    None
                }
            }
        });
        Self {
            stream: boxed.boxed(),
            keep_alive: None,
        }
    }

    /// 启用 keep-alive，定时发送注释行防止连接超时。
    pub fn keep_alive(mut self, interval: Duration) -> Self {
        self.keep_alive = Some(interval);
        self
    }
}

impl IntoResponse for SseResponse {
    fn into_response(self) -> axum::response::Response {
        let Self { stream, keep_alive } = self;
        let sse = axum::response::sse::Sse::new(stream);
        if let Some(d) = keep_alive {
            sse.keep_alive(axum::response::sse::KeepAlive::new().interval(d))
                .into_response()
        } else {
            sse.into_response()
        }
    }
}

/// 将流元素统一转为 `Result<SseEvent, E>`。
///
/// 使 `SseResponse::new` 同时接受 `Stream<Item = SseEvent>`
/// 和 `Stream<Item = Result<SseEvent, E>>`。
pub trait IntoSseEventResult<E> {
    fn into_result(self) -> Result<SseEvent, E>;
}

impl IntoSseEventResult<Infallible> for SseEvent {
    fn into_result(self) -> Result<SseEvent, Infallible> {
        Ok(self)
    }
}

impl<E> IntoSseEventResult<E> for Result<SseEvent, E> {
    fn into_result(self) -> Result<SseEvent, E> {
        self
    }
}
