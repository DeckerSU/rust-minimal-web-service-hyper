use bytes::Bytes;
use hyper::{
    body::to_bytes,
    service::{make_service_fn, service_fn},
    Body, Request, Server, server::conn::AddrStream,
};
use route_recognizer::Params;
use router::Router;
use std::{sync::{Arc, Mutex}, net::SocketAddr};

mod handler;
mod router;

type Response = hyper::Response<hyper::Body>;
type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Clone, Debug)]
pub struct AppState {
    pub state_thing: String,
    pub counter: u64,
}

#[tokio::main]
async fn main() {

    let mut router: Router = Router::new();

    router.get("/test", Box::new(handler::test_handler));
    router.get("/counter", Box::new(handler::counter_handler));
    router.post("/send", Box::new(handler::send_handler));
    router.get("/params/:some_param", Box::new(handler::param_handler));

    let shared_router = Arc::new(router);

    // Create our initial AppState
    let app_state = Arc::new(Mutex::new(AppState {
        state_thing: "state".to_string(),
        counter: 0,
    }));

    // Create a closure that creates a service from our handler and state
    let new_service = make_service_fn(move |conn: &AddrStream| {

        let sock_addr = conn.remote_addr();

        let router_capture = shared_router.clone();
        let app_state = app_state.clone();

        async move {
            Ok::<_, Error>(service_fn(move |req| {
                route(router_capture.clone(), req, app_state.clone(), sock_addr.clone())
            }))
        }
    });

    let addr = "0.0.0.0:8080".parse().expect("address creation works");
    let server = Server::bind(&addr).serve(new_service);
    println!("Listening on http://{}", addr);
    let _ = server.await;

}

async fn route(
    router: Arc<Router>,
    req: Request<hyper::Body>,
    app_state: Arc<Mutex<AppState>>,
    sock_addr: SocketAddr,
) -> Result<Response, Error> {

    let found_handler = router.route(req.uri().path(), req.method());
    let resp = found_handler
        .handler
        .invoke(Context::new(app_state, req, found_handler.params, sock_addr))
        .await;
    Ok(resp)
}

#[derive(Debug)]
pub struct Context {
    pub state: Arc<Mutex<AppState>>,
    pub req: Request<Body>,
    pub params: Params,
    body_bytes: Option<Bytes>,
    pub sock_addr: SocketAddr,
}

impl Context {
    pub fn new(state: Arc<Mutex<AppState>>, req: Request<Body>, params: Params, sock_addr: SocketAddr) -> Context {
        Context {
            state,
            req,
            params,
            body_bytes: None,
            sock_addr,
        }
    }

    pub async fn body_json<T: serde::de::DeserializeOwned>(&mut self) -> Result<T, Error> {
        let body_bytes = match self.body_bytes {
            Some(ref v) => v,
            _ => {
                let body = to_bytes(self.req.body_mut()).await?;
                self.body_bytes = Some(body);
                self.body_bytes.as_ref().expect("body_bytes was set above")
            }
        };
        Ok(serde_json::from_slice(&body_bytes)?)
    }
}
