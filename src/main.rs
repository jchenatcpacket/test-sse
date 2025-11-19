use actix_web::{App, HttpRequest, HttpResponse, HttpServer, Responder, get, web};
use actix_web_lab::{extract::Path, sse};
use futures_util::stream;
use std::{convert::Infallible, time::Duration};
use tokio::time::sleep;
use log::{debug};
use tokio::sync::broadcast;

#[get("/hello/{name}")]
async fn greet(name: web::Path<String>) -> impl Responder {
    format!("Hello {}!", name)
}

#[get("/countdown/{n:\\d+}")]
async fn countdown_from(Path(n): Path<u32>, req: HttpRequest) -> impl Responder {
    // note: a more production-ready implementation might want to use the lastEventId header
    // sent by the reconnecting browser after the _retry_ period
    debug!("lastEventId: {:?}", req.headers().get("Last-Event-ID"));

    common_countdown(n.try_into().unwrap())
}

fn common_countdown(n: i32) -> impl Responder {
    let countdown_stream = stream::unfold((false, n), |(started, n)| async move {
        // allow first countdown value to yield immediately
        if started {
            sleep(Duration::from_secs(1)).await;
        }

        if n > 0 {
            let data = sse::Data::new(n.to_string())
                .event("countdown")
                .id(n.to_string());

            Some((Ok::<_, Infallible>(sse::Event::Data(data)), (true, n - 1)))
        } else {
            None
        }
    });

    sse::Sse::from_stream(countdown_stream).with_retry_duration(Duration::from_secs(5))
}

#[get("/set_atomic_countdown/{n:\\d+}")]
async fn set_atomic_countdown(Path(n): Path<i32>, tx: web::Data<broadcast::Sender<i32>>) -> HttpResponse {
    tokio::spawn(async move {
        for i in (0..n).rev() {
            match tx.send(i) {
                Ok(_) => {
                }
                Err(_) => {
                    // No active receivers, do nothing let value pass
                }
            }
            sleep(Duration::from_secs(5)).await;
        }
    });

    HttpResponse::Ok().body(format!("good: {}", n))
}

#[get("/listen_atomic_countdown")]
async fn listen_atomic_countdown(tx: web::Data<broadcast::Sender<i32>>) -> impl Responder {
    let rx = tx.subscribe();

    let countdown_stream = stream::unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Ok(value) => {
                let data = sse::Data::new(value.to_string())
                    .event("countdown")
                    .id(value.to_string());
                Some((Ok::<_, Infallible>(sse::Event::Data(data)), rx))
            }
            Err(_) => None,
        }
    });

    sse::Sse::from_stream(countdown_stream).with_retry_duration(Duration::from_secs(5))
}

#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {
    let (tx, _) = broadcast::channel::<i32>(16);
    HttpServer::new(move || {
        App::new()
            .app_data(actix_web::web::Data::new(tx.clone()))
            .service(greet)
            .service(countdown_from)
            .service(set_atomic_countdown)
            .service(listen_atomic_countdown)
            .route("/ping", web::get().to(|| async { "pong" }))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
