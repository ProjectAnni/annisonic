use std::pin::Pin;
use std::task::{Context, Poll};

use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error};
use actix_utils::future::{ok, Ready};
use std::future::Future;
use actix_web::web::Query;
use serde::Deserialize;
use actix_web::dev::{Transform, Service};

#[derive(Debug, Deserialize)]
struct Auth {
    #[serde(rename = "u")]
    username: String,
    #[serde(rename = "p")]
    password: Option<String>,
    #[serde(rename = "t", default)]
    token: String,
    #[serde(rename = "s", default)]
    salt: String,
    #[serde(rename = "c", default)]
    client: String,
    #[serde(rename = "v", default)]
    version: String,
}

pub struct SonicAuth;

impl<S, B> Transform<S, ServiceRequest> for SonicAuth
    where
        S: Service<ServiceRequest, Response=ServiceResponse<B>, Error=Error>,
        S::Future: 'static,
        B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = SonicAuthMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(SonicAuthMiddleware { service })
    }
}

pub struct SonicAuthMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for SonicAuthMiddleware<S>
    where
        S: Service<ServiceRequest, Response=ServiceResponse<B>, Error=Error>,
        S::Future: 'static,
        B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output=Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let query = Query::<Auth>::from_query(req.query_string());
        match query {
            Ok(query) => {
                let query = query.into_inner();
                // t = md5(password+s)
                if query.username == std::env::var("ANNI_USER").unwrap()
                    && match query.password {
                    None => { query.token == format!("{:x}", md5::compute(std::env::var("ANNI_PASSWD").unwrap() + &query.salt)) }
                    Some(password) => {
                        let password = if password.starts_with("enc:") { &password[4..] } else { &password };
                        password == std::env::var("ANNI_PASSWD_HEX").unwrap()
                    }
                } {
                    let fut = self.service.call(req);
                    Box::pin(async {
                        let res = fut.await?;
                        Ok(res)
                    })
                } else {
                    // wrong password
                    Box::pin(async {
                        let res = req.error_response(std::io::Error::new(std::io::ErrorKind::InvalidInput, ""));
                        Ok(res)
                    })
                }
            }
            Err(_) => Box::pin(async {
                let res = req.error_response(std::io::Error::new(std::io::ErrorKind::InvalidInput, ""));
                Ok(res)
            })
        }
    }
}