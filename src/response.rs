pub fn ok(mut body: String) -> String {
    body.insert_str(0, r#"<subsonic-response xmlns="http://subsonic.org/restapi" status="ok" version="1.15.0">
"#);
    body.push_str(r#"
</subsonic-response>"#);
    body
}