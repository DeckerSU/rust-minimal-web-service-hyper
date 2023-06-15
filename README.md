# rust-minimal-web-service-hyper

An example of a minimal web service in Rust using hyper

Run with `make dev`

URLs to call:

```bash
curl http://localhost:8080/test

curl http://localhost:8080/params/1234

curl -X POST http://localhost:8080/send -d '{"name": "chip", "active": true }'

```

### Useful documentation

- https://blog.logrocket.com/a-minimal-web-service-in-rust-using-hyper/
- https://hyper.rs/
- https://github.com/hyperium/hyper/

### F.A.Q.

- What's the difference between `hyper::Server` and `tokio::net::TcpListener`?

    `hyper::Server` and `tokio::net::TcpListener` are two different concepts. 

    `tokio::net::TcpListener` is used to accept incoming TCP connections. It is a low level building block in Rust for building TCP servers. `TcpListener` applications are built on top of `tokio` and used in conjunction with other libraries like `hyper` for building high-performance network applications.

    On the other hand, `hyper::Server` is a higher-level HTTP server framework built using `tokio` and is intended to provide a more user-friendly interface for building HTTP servers. It provides a number of abstractions and building blocks for handling incoming HTTP requests and sending responses. One of these building blocks is the `TcpListener` which is used for accepting incoming TCP connections. 

    Therefore, if you want to build an HTTP server using hyper, you would create a `hyper::Server` instance which internally uses `tokio::net::TcpListener` to listen for incoming requests.

    Also, example of working `tokio::net::TcpListener` exists in Hyper [1.0 guides](https://hyper.rs/guides/1/server/hello-world/) and `hyper::Server` introduced in [0.14 guides](https://hyper.rs/guides/0.14/server/hello-world/).

- Here is an example of how to change the `AppState` in Hyper

    ```rust
    use hyper::service::{make_service_fn, service_fn};
    use hyper::{Body, Request, Response, Server};
    use std::convert::Infallible;
    use std::sync::{Arc, Mutex};

    // Define our state struct
    struct AppState {
        counter: u32,
    }

    // This is our service handler. It receives a Request, routes on its
    // path, and returns a Future of a Response.
    async fn handle_request(
        req: Request<Body>,
        state: Arc<Mutex<AppState>>,
    ) -> Result<Response<Body>, Infallible> {
        match req.uri().path() {
            // If the request is for /increment, increment the counter in AppState
            "/increment" => {
                let mut lock = state.lock().unwrap();
                lock.counter += 1;
                let body = format!("Counter is now at {}", lock.counter);
                Ok(Response::new(Body::from(body)))
            }
            // If the request is for anything else, return a 404 Not Found
            _ => {
                let body = "Not Found".to_string();
                let response = Response::builder()
                    .status(404)
                    .body(Body::from(body))
                    .unwrap();
                Ok(response)
            }
        }
    }

    #[tokio::main]
    async fn main() {
        // Create our initial AppState
        let app_state = Arc::new(Mutex::new(AppState { counter: 0 }));

        // Create a closure that creates a service from our handler and state
        let service = make_service_fn(move |_| {
            let app_state = app_state.clone();
            async {
                Ok::<_, Infallible>(service_fn(move |req| {
                    let state = app_state.clone();
                    handle_request(req, state)
                }))
            }
        });

        // Create a Hyper server and start serving requests
        let addr = ([127, 0, 0, 1], 3000).into();
        let server = Server::bind(&addr).serve(service);
        println!("Listening on http://{}", addr);
        if let Err(e) = server.await {
            eprintln!("server error: {}", e);
        }
    }
    ```

