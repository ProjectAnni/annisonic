mod response;
mod auth;

use actix_web::{HttpServer, Responder, HttpResponse, get, App};
use actix_web::middleware::Logger;
use crate::auth::SonicAuth;

#[get("/ping")]
async fn ping() -> impl Responder {
    HttpResponse::Ok()
        .content_type("application/xml")
        .body(response::response(String::new()))
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    HttpServer::new(move || {
        App::new()
            // .app_data(state.clone())
            .wrap(Logger::default())
            .wrap(SonicAuth)
            .service(ping)
    })
        .bind("localhost:1710")?
        .run()
        .await?;
    Ok(())
}
