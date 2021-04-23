use actix_web::{dev, Result, body::Body};
use actix_web::middleware::ErrorHandlerResponse;
use actix_web::dev::ResponseBody;
use actix_web::http::StatusCode;

pub fn ok(mut body: String) -> String {
    body.insert_str(0, r#"<subsonic-response xmlns="http://subsonic.org/restapi" status="ok" version="1.15.0">
"#);
    body.push_str(r#"
</subsonic-response>"#);
    body
}

pub fn gone<B>(res: dev::ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>> {
    let url = res.request().uri().to_string();
    let mut res = res.map_body(|_, _| ResponseBody::Body(Body::from(format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<subsonic-response xmlns="http://subsonic.org/restapi" status="failed" version="1.15.0">
   <error code="30" message="{}"/>
</subsonic-response>"#, url))).into_body()
    );
    *res.response_mut().status_mut() = StatusCode::OK;
    Ok(ErrorHandlerResponse::Response(res))
}