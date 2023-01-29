use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    thread, vec,
};

use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use once_cell::sync::Lazy;
use tokio::sync::mpsc::{Receiver, Sender};
use url::Url;

use crate::gui::MultiPlot;
pub(crate) type QueryParam = Vec<(String, String)>;

pub(crate) static QUERY_SENDER: Lazy<Mutex<Option<Sender<QueryParam>>>> =
    Lazy::new(|| Mutex::new(None));

async fn append_records(r: Request<Body>) -> Result<Response<Body>, String> {
    let query: Vec<_> = r
        .uri()
        .query()
        .map(|v| {
            url::form_urlencoded::parse(v.as_bytes())
                .into_owned()
                .collect()
        })
        .unwrap_or_default();
    let sender = QUERY_SENDER.lock().unwrap().clone();
    // don't hold a mutex while `await` waiting
    if let Some(sender) = sender {
        sender
            .send(query)
            .await
            .map_err(|err| format!("Channel was closed by receiver!{err}"))?;
    } else {
        return Err("No sender is set, can't send query to gui thread".to_string());
    }
    Ok(Response::new(Body::empty()))
}

async fn start_server(port: u16) {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let make_svc = make_service_fn(|_conn| async {
        // service_fn converts our function into a `Service`
        Ok::<_, String>(service_fn(append_records))
    });
    let server = Server::bind(&addr).serve(make_svc);
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}

pub fn start_threaded_server(port: u16) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async { start_server(port).await });
    })
}
