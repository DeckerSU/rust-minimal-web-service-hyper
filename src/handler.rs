use crate::{Context, Response};
use hyper::{StatusCode, Body};
use serde::Deserialize;

pub async fn test_handler(ctx: Context) -> String {
    let state = ctx.state.lock().unwrap();
    format!("test called, state_thing was: {}", state.state_thing)
}

pub async fn counter_handler(ctx: Context) -> Response {
    let mut state = ctx.state.lock().unwrap();

    state.counter = state.counter + 1;
    let body = format!("Socket address: {}\nCounter is now at {}", ctx.sock_addr, state.counter);

    hyper::Response::builder()
        .status(200)
        .header("Content-Type", "text/plain")
        .body(Body::from(body))
        .unwrap()
}

#[derive(Deserialize)]
struct SendRequest {
    name: String,
    active: bool,
}

pub async fn send_handler(mut ctx: Context) -> Response {
    let body: SendRequest = match ctx.body_json().await {
        Ok(v) => v,
        Err(e) => {
            return hyper::Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(format!("could not parse JSON: {}", e).into())
                .unwrap()
        }
    };

    Response::new(
        format!(
            "send called with name: {} and active: {}",
            body.name, body.active
        )
        .into(),
    )
}

pub async fn param_handler(ctx: Context) -> String {
    let param = match ctx.params.find("some_param") {
        Some(v) => v,
        None => "empty",
    };
    format!("param called, param was: {}", param)
}
