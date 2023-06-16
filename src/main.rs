use hyper::{
    body::to_bytes,
    service::{make_service_fn, service_fn},
    Body, Request, Server, server::{conn::{AddrStream, AddrIncoming}, accept::Accept},
};
use route_recognizer::Params;
use router::Router;
use rustls::ServerConfig;
// use tokio_rustls::TlsAcceptor;
use std::{sync::{Arc, Mutex}, net::SocketAddr, fs::File, io::BufReader, pin::Pin};
use rustls_pemfile as pemfile;

mod handler;
mod router;
mod tls;

type Response = hyper::Response<hyper::Body>;
type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Clone, Debug)]
pub struct AppState {
    pub state_thing: String,
    pub counter: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {

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

    // openssl req -x509 -newkey rsa:4096 -nodes -keyout key.pem -out cert.pem -days 365

    // https://github.com/svenstaro/miniserve/blob/990bfaebdcc11f01a609d2034fb7876f4799f681/src/config.rs
    // https://github.com/KomodoPlatform/atomicDEX-API/pull/1861/files#diff-25baee98803ce2de15ee914b54676998e353d0fed81e51c5efb83756e260cabd
    // https://github.com/SergioBenitez/Rocket/blob/9a9cd76c0121f46765ff0df9ef81e36563a2a31f/core/http/src/tls/listener.rs#L96
    // http://zderadicka.eu/hyper-and-tls/
    // https://github.com/izderadicka/audioserve

    // Create a `rustls::ServerConfig` with the self-signed certificate and key
    let cert_file = &mut BufReader::new(File::open("cert.pem")?);
    let key_file = &mut BufReader::new(File::open("key.pem")?);
    let cert_chain = pemfile::certs(cert_file)?;
    let key = pemfile::read_all(key_file)?.into_iter().find_map(|item| match item {
        pemfile::Item::RSAKey(key)
        | pemfile::Item::PKCS8Key(key)
        | pemfile::Item::ECKey(key) => Some(key),
        _ => None,
    }).ok_or("No supported private key in file".to_string())?;

    // Create a `rustls::ServerConfig` with the self-signed certificate and key
    let tls_config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert_chain.into_iter().map(rustls::Certificate).collect(), rustls::PrivateKey(key))?;

    
    //let new_service = make_service_fn(move |conn: &AddrStream| {

    // Create a closure that creates a service from our handler and state
    let new_service = make_service_fn(move |conn: &tls::TlsStream| {

        let sock_addr = conn.remote_addr();

        let router_capture = shared_router.clone();
        let app_state = app_state.clone();

        async move {
            Ok::<_, Error>(service_fn(move |req| {
                route(router_capture.clone(), req, app_state.clone(), sock_addr.clone())
            }))
        }
    });

    // http
    // let addr = "0.0.0.0:8080".parse().expect("address creation works");
    // let server = Server::bind(&addr).serve(new_service);

    // let acceptor = TlsAcceptor::from(Arc::new(tls_config));
    // We can't use acceptor above, because
    // the trait `hyper::server::accept::Accept` is not implemented for `tokio_rustls::TlsAcceptor`
    // https://github.com/hyperium/hyper-tls/issues/25#issuecomment-575635447,
    // so we will use TlsAcceptor here.

    // https
    let addr = "0.0.0.0:8443".parse().expect("address creation works");
    let incoming = AddrIncoming::bind(&addr)?;
    let tls_acceptor = tls::tls_acceptor(Arc::new(tls_config), incoming);
    let server = Server::builder(tls_acceptor).serve(new_service);

    println!("Listening on https://{}", addr);
    let _ = server.await;

    Ok(())

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
    body_bytes: Option<hyper::body::Bytes>,
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
