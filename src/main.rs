use axum::{
    body::Body,
    extract::Request,
    http::{header, Method, StatusCode},
    response::Response,
    routing::any,
    Router,
};
use tokio::net::TcpListener;

const UPSTREAM_BASE: &str = "https://api.anthropic.com";
const LISTEN_ADDR: &str = "127.0.0.1:47821";

async fn proxy(req: Request) -> Result<Response, StatusCode> {
    let (parts, body) = req.into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    eprintln!("[req] {} {}", parts.method, parts.uri);

    let mut forward_bytes = bytes.to_vec();

    if parts.method == Method::POST && parts.uri.path() == "/v1/messages" {
        match serde_json::from_slice::<serde_json::Value>(&bytes) {
            Ok(mut json) => match load_append_text().await {
                Some(text) => {
                    append_to_system(&mut json, &text);
                    match serde_json::to_vec(&json) {
                        Ok(b) => {
                            eprintln!("[append] applied ({} bytes)", text.len());
                            forward_bytes = b;
                        }
                        Err(e) => eprintln!("[append] re-serialize failed: {e}"),
                    }
                }
                None => eprintln!("[append] no append file"),
            },
            Err(e) => eprintln!("[append] json parse failed: {e}"),
        }
    }

    let path_and_query = parts
        .uri
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("");
    let upstream_url = format!("{UPSTREAM_BASE}{path_and_query}");

    let client = reqwest::Client::new();
    let mut req_builder = client.request(parts.method, &upstream_url);
    for (k, v) in parts.headers.iter() {
        if k == header::HOST || k == header::CONTENT_LENGTH {
            continue;
        }
        req_builder = req_builder.header(k, v);
    }

    let upstream_res = req_builder
        .body(forward_bytes)
        .send()
        .await
        .map_err(|e| {
            eprintln!("upstream request failed: {e}");
            StatusCode::BAD_GATEWAY
        })?;

    let status = upstream_res.status();
    let headers = upstream_res.headers().clone();
    let stream = upstream_res.bytes_stream();

    let mut builder = Response::builder().status(status.as_u16());
    for (k, v) in headers.iter() {
        if k == header::CONTENT_LENGTH || k == header::TRANSFER_ENCODING {
            continue;
        }
        builder = builder.header(k, v);
    }
    builder
        .body(Body::from_stream(stream))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn load_append_text() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let path = format!("{home}/.config/claude/claude-shim.md");
    tokio::fs::read_to_string(&path).await.ok()
}

fn append_to_system(body: &mut serde_json::Value, text: &str) {
    match body.get_mut("system") {
        Some(serde_json::Value::String(s)) => {
            s.push_str("\n\n");
            s.push_str(text);
        }
        Some(serde_json::Value::Array(arr)) => {
            arr.push(serde_json::json!({"type": "text", "text": text}));
        }
        _ => {
            body["system"] = serde_json::Value::String(text.to_string());
        }
    }
}

#[tokio::main]
async fn main() {
    let app = Router::new().fallback(any(proxy));
    let listener = TcpListener::bind(LISTEN_ADDR)
        .await
        .expect("bind failed");
    println!("listening on http://{LISTEN_ADDR}");
    axum::serve(listener, app).await.expect("server error");
}
