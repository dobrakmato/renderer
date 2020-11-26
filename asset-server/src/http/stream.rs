use crate::http::models::Event;
use actix_web::rt::time::{interval_at, Instant};
use actix_web::web::{Bytes, Data};
use actix_web::{Error, HttpResponse, Responder};
use futures::{Stream, StreamExt};
use log::error;
use once_cell::sync::OnceCell;
use std::pin::Pin;
use std::sync::Mutex;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::sync::mpsc::{channel, Receiver, Sender};

pub async fn new_client(broadcaster: Data<Mutex<Broadcaster>>) -> impl Responder {
    let rx = broadcaster.lock().unwrap().new_client();

    HttpResponse::Ok()
        .header("content-type", "text/event-stream")
        .streaming(rx)
}

static BROADCASTER: OnceCell<Data<Mutex<Broadcaster>>> = OnceCell::new();

pub fn create_event_stream() -> Data<Mutex<Broadcaster>> {
    let broadcaster = Broadcaster::create();
    BROADCASTER.get_or_init(|| broadcaster.clone());
    broadcaster
}

/// Published the specified event to global event stream.
pub fn publish_server_event(event: Event) {
    if let Some(t) = BROADCASTER.get() {
        let json = match serde_json::to_string(&event) {
            Ok(json) => json,
            Err(t) => {
                error!("Cannot convert event: {:?}", t);
                return;
            }
        };

        t.lock().unwrap().send(&json);
    }
}

pub struct Broadcaster {
    clients: Vec<Sender<Bytes>>,
}

impl Broadcaster {
    pub fn create() -> Data<Mutex<Self>> {
        let me = Data::new(Mutex::new(Broadcaster::new()));
        Broadcaster::spawn_ping(me.clone());
        me
    }

    fn new() -> Self {
        Broadcaster {
            clients: Vec::new(),
        }
    }

    fn spawn_ping(me: Data<Mutex<Self>>) {
        actix_web::rt::spawn(async move {
            let mut task = interval_at(Instant::now(), Duration::from_secs(10));
            while task.next().await.is_some() {
                me.lock().unwrap().remove_stale_clients();
            }
        })
    }

    fn remove_stale_clients(&mut self) {
        let mut ok_clients = Vec::new();
        for client in self.clients.iter() {
            let result = client.clone().try_send(Bytes::from("data: ping\n\n"));

            if let Ok(()) = result {
                ok_clients.push(client.clone());
            }
        }
        self.clients = ok_clients;
    }

    fn new_client(&mut self) -> Client {
        let (tx, rx) = channel(100);

        tx.clone()
            .try_send(Bytes::from("data: connected\n\n"))
            .unwrap();

        self.clients.push(tx);
        Client(rx)
    }

    fn send(&self, msg: &str) {
        let msg = Bytes::from(["data: ", msg, "\n\n"].concat());

        for client in self.clients.iter() {
            client.clone().try_send(msg.clone()).unwrap_or(());
        }
    }
}

// wrap Receiver in own type, with correct error type
struct Client(Receiver<Bytes>);

impl Stream for Client {
    type Item = Result<Bytes, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.0).poll_next(cx) {
            Poll::Ready(Some(v)) => Poll::Ready(Some(Ok(v))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
