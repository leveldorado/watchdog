use std::io;

pub struct Proxy {
    ssl: bool,
    targets: Vec<proxy_path>,
    server_name: String,
}

struct proxy_path {
    path: String,
    app_name: String,
    websocket: bool,
}

pub fn set_proxy(p: Proxy) -> Result<(), io::Error> {
    return Ok(());
}
