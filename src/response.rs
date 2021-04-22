use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename = "subsonic-response")]
pub enum SonicResponse {
//
}

pub fn response(mut body: String) -> String {
    body.insert_str(0, r#"<subsonic-response xmlns="http://subsonic.org/restapi" status="ok" version="1.15.0">
"#);
    body.push_str(r#"
</subsonic-response>"#);
    body
}