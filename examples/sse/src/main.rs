use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;
use webr::prelude::*;
use webr::{SseEvent, SseResponse, Error};

// ─── SSE Controller ─────────────────────────────────────────

#[controller]
pub struct SseController;

#[controller]
impl SseController {
    /// 首页：嵌入 JS EventSource 客户端
    #[get("/")]
    async fn index(&self) -> axum::response::Html<&'static str> {
        axum::response::Html(
            r#"<!DOCTYPE html>
<html lang="zh">
<head>
    <meta charset="UTF-8">
    <title>SSE Demo</title>
    <style>
        body { font-family: system-ui, sans-serif; max-width: 640px; margin: 40px auto; padding: 0 20px; background: #fafafa; color: #333; }
        h1 { font-size: 1.5rem; margin-bottom: 0.5rem; }
        .subtitle { color: #888; margin-bottom: 1.5rem; }
        .section { margin-bottom: 2rem; }
        .section h2 { font-size: 1rem; color: #555; border-bottom: 1px solid #e0e0e0; padding-bottom: 4px; margin-bottom: 8px; }
        .log { font-family: 'Courier New', monospace; font-size: 0.85rem; background: #fff; border: 1px solid #e0e0e0; border-radius: 4px; padding: 12px; min-height: 80px; white-space: pre-wrap; word-break: break-all; }
        .log .event { color: #0969da; }
        .log .data { color: #1a7f37; }
        .log .meta { color: #888; }
        button { padding: 6px 16px; border: 1px solid #d0d0d0; border-radius: 4px; background: #fff; cursor: pointer; font-size: 0.85rem; margin-right: 8px; }
        button:hover { background: #f0f0f0; }
        button.active { background: #0969da; color: #fff; border-color: #0969da; }
    </style>
</head>
<body>
    <h1>SSE Demo</h1>
    <p class="subtitle">Server-Sent Events · webr framework</p>

    <div class="section">
        <h2>Batch Events <span style="color:#888;font-weight:normal">— GET /events</span></h2>
        <button onclick="loadBatch()">Send Batch</button>
        <div id="batch-log" class="log" style="margin-top:8px"></div>
    </div>

    <div class="section">
        <h2>Continuous Stream <span style="color:#888;font-weight:normal">— GET /stream</span></h2>
        <button id="stream-btn" onclick="toggleStream()">Connect</button>
        <div id="stream-log" class="log" style="margin-top:8px"></div>
    </div>

    <script>
        function appendLog(el, type, text) {
            const span = document.createElement('span');
            span.className = type;
            span.textContent = text + '\n';
            el.appendChild(span);
            el.scrollTop = el.scrollHeight;
        }

        function loadBatch() {
            const log = document.getElementById('batch-log');
            log.innerHTML = '';
            const es = new EventSource('/events');
            es.onmessage = (e) => {
                appendLog(log, 'data', '← ' + e.data);
                if (e.lastEventId) appendLog(log, 'meta', '  id: ' + e.lastEventId);
            };
            es.addEventListener('chat', (e) => {
                appendLog(log, 'event', '← [chat] ' + e.data);
            });
            es.onerror = () => { es.close(); appendLog(log, 'meta', '— stream closed —'); };
        }

        let streamEs = null;
        function toggleStream() {
            const btn = document.getElementById('stream-btn');
            const log = document.getElementById('stream-log');
            if (streamEs) {
                streamEs.close();
                streamEs = null;
                btn.textContent = 'Connect';
                btn.classList.remove('active');
                appendLog(log, 'meta', '— disconnected —');
            } else {
                streamEs = new EventSource('/stream');
                btn.textContent = 'Disconnect';
                btn.classList.add('active');
                streamEs.onmessage = (e) => {
                    appendLog(log, 'data', '← ' + e.data);
                };
                streamEs.onerror = () => {
                    appendLog(log, 'meta', '— connection error —');
                };
            }
        }
    </script>
</body>
</html>"#,
        )
    }

    /// 批量发送 SSE 事件后自动关闭连接
    #[get("/events")]
    async fn events(&self) -> SseResponse {
        let events = vec![
            SseEvent::new(r#"{"text":"hello"}"#),
            SseEvent::new(r#"{"text":"world"}"#).event("chat").id("msg-1"),
            SseEvent::new(r#"{"text":"done"}"#).id("msg-2"),
        ];
        SseResponse::new(tokio_stream::iter(events))
    }

    /// 持续推送 SSE 事件流
    #[get("/stream")]
    async fn stream(&self) -> SseResponse {
        let interval = tokio::time::interval(Duration::from_secs(1));
        let stream = IntervalStream::new(interval).map(|_| {
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis();
            SseEvent::new(format!(r#"{{"ts":{ts}}}"#)).id(ts.to_string())
        });
        SseResponse::new(stream).keep_alive(Duration::from_secs(15))
    }
}

// ─── 启动入口 ────────────────────────────────────────────────

#[webr::main]
async fn main(_app: &mut webr::AppBuilder) -> std::result::Result<(), Error> {
    println!("SSE Demo: http://localhost:3000");
    Ok(())
}
