extern crate pretty_env_logger;
extern crate log;
extern crate actix;
extern crate actix_web;
extern crate actix_net;
extern crate askama;
extern crate icc;
extern crate ctrlc;

use actix_web::http::{header, Method, StatusCode, HttpTryFrom};
use actix_web::middleware::{Middleware, Finished, Response, Started};
use actix_web::middleware::session::{self, RequestSession};
use actix_web::{error, fs, middleware, pred, server, App, Error, HttpRequest, HttpResponse, Path, Result, ws};
use actix::prelude::*;
use askama::Template;
use serde::{Serialize, Deserialize};
use log::{error, info, debug};
use std::env;
use std::sync::{Arc, RwLock};
use icc::util::config::{config, Config};

struct GlobalData {
    pub is_down : bool
}

impl GlobalData {
    pub fn new() -> Arc<RwLock<GlobalData>> {
        Arc::new(RwLock::new(GlobalData { is_down: false }))
    }
}

// Default headers middleware
struct SetDefaultHeaders;

impl<S> Middleware<S> for SetDefaultHeaders {
    fn start(&self, req: &HttpRequest<S>) -> Result<Started> {
        Ok(Started::Done)
    }

    fn response(&self, _req: &HttpRequest<S>, mut resp: HttpResponse) -> Result<Response> {
        resp.headers_mut().insert(
            header::HeaderName::try_from("server").unwrap(),
            header::HeaderValue::from_static("icc"));
        Ok(Response::Done(resp))
    }

    fn finish(&self, _req: &HttpRequest<S>, _resp: &HttpResponse) -> Finished {
        Finished::Done
    }
}

// Websocket stream
#[derive(Message)]
pub struct Message(pub String);

struct Ws;

#[derive(Message)]
struct Connect {
    pub addr: Recipient<Message>
}

impl Actor for Ws {
    type Context = ws::WebsocketContext<Self, Arc<RwLock<GlobalData>>>;
}

impl StreamHandler<ws::Message, ws::ProtocolError> for Ws {
    fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {
        debug!("WS: {:?}", msg);
        match msg {
            ws::Message::Ping(msg) => ctx.pong(&msg),
            ws::Message::Text(text) => ctx.text(text),
            ws::Message::Binary(bin) => ctx.binary(bin),
            _ => (),
        }
    }
}

impl Handler<Connect> for Ws {
    type Result = ();

    fn handle(&mut self, msg: Connect, ctx: &mut Self::Context) {
        debug!("Connect message being handled");
        self.init(msg.addr);
    }
}

impl Ws {
    fn init(&self, addr : Recipient<Message>) {
        addr.do_send(Message("test".to_owned()));
    }
}

// Index template

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate<'a> {
    is_down: &'a str
}

fn main() {
    let config : Config = config();
    let sys = actix::System::new("icc-web");

    setup();

    let data = GlobalData::new();

    let app = move || {
        let d = data.clone();
        App::with_state(d)
            .middleware(SetDefaultHeaders) // Sets 'server' header
            .resource("/xaxa", |r| r.method(Method::GET).f(|req| {
                info!("{:?}", req);
                match *req.method() {
                    Method::GET => HttpResponse::Ok(),
                    _ => HttpResponse::NotFound(),
                }
            }))
            .resource("/1", |r| r.method(Method::GET).f(|req| {
                HttpResponse::Ok()
                    .content_type("text/html; charset=utf-8")
                    .header("server", "icc")
                    .body(format!("1"))
                // .finish()
            }))

            .resource("/s/", |r| {
                r.f(|req| {
                    ws::start(req, Ws)
                })
            })

            .resource("/", |r| {
                r.method(Method::GET).f(move |req| {
                    let state : &Arc<RwLock<GlobalData>> = req.state();
                    let is_down = state.read().unwrap().is_down;

                    if is_down == true {
                        state.write().unwrap().is_down = false;
                    } else {
                        state.write().unwrap().is_down = true;
                    }

                    let payload = IndexTemplate {is_down: format!("{}", is_down).as_str()}.render().unwrap();


                    HttpResponse::Ok()
                        .content_type("text/html; charset=utf-8")
                        .header("server", "icc")
                        .body(payload)
                })
            })
    };


    let server = server::new(app)
        .bind(config.bind_address.expect("Bind address is not specified"))
        .unwrap()
        .shutdown_timeout(5)
        .disable_signals();
    let server_addr = server.start();

    let handleicc = HandleIcc {http_server: server_addr}.start();
    handle_exit(handleicc);

    sys.run();
}

struct HandleIcc {
    http_server : Addr<actix_net::server::Server>
}

impl actix::Actor for HandleIcc{
    type Context = Context<Self>;
}

struct IccShutdown;

impl actix::Message for IccShutdown {
    type Result = usize;
}

impl Handler<IccShutdown> for HandleIcc {
    type Result = usize;

    fn handle(&mut self, msg: IccShutdown, ctx: &mut Context<Self>) -> usize{
        self.http_server.do_send(actix_web::server::StopServer {graceful: true});
        info!("Stopping ICC services");
        0
    }
}

fn handle_exit(addr : Addr<HandleIcc>) {
    ctrlc::set_handler(move || {
        addr.do_send(IccShutdown {});
        std::thread::sleep(std::time::Duration::from_secs(1));
        std::process::exit(1);
    }).expect("Unable to set SIGINT handler");
}


#[cfg(debug_assertions)]
fn setup() {
    env::set_var("RUST_LOG", "trace,tokio_reactor=info,mio=info,actix_net=info,actix_web=info");
    pretty_env_logger::init();
}

#[cfg(not(debug_assertions))]
fn setup() {

}