use std::io;
use app::{App, Volume};
use app::containers;
use app::config::OkErr;
use std::fs::File;
use std::io::Write;


#[derive(Default, Deserialize)]
pub struct Proxy {
    ssl: bool,
    targets: Vec<ProxyPath>,
    server_name: String,
}


impl Proxy {
    fn build_nginx_conf(&self) -> Result<String, io::Error> {
        let mut ssl_cache = "";
        let mut port = "80";
        let mut ssl = "";
        if self.ssl {
            ssl_cache = SSL_CACHE_DIRECTIVE;
            port = "403 ssl";
            ssl = SSL_DIRECTIVE;
        }
        let mut location = String::new();
        for loc in &self.targets {
            match containers::get_app(loc.app_name.clone()) {
                Ok(app) => {
                    let mut ws = "";
                    if loc.websocket {
                        ws = WS_PROXY_DIRECTIVE;
                    }
                    let target = format!("
     location {} {{
         proxy_path {};
         {}
     }}
",
                                         loc.path,
                                         ws,
                                         app.addr());
                    location.push_str(target.as_ref());
                }
                Err(e) => return Err(e),
            }
        }
        return Ok(format!("
worker_processes auto;
http {{
    {}
    server {{
    listen              {};
    server_name         {};
    keepalive_timeout   60;
    {}
    location /health {{
        return 200;
    }}
    {}
   }}
}}
",
                          ssl_cache,
                          port,
                          self.server_name,
                          ssl,
                          location));
    }
    fn build_app(&self) -> App {
        let mut app = App::new();
        app.volumes = vec![Volume {
                               host_path: SSL_CERT_PATH.to_string(),
                               container_path: SSL_CERT_PATH.to_string(),
                           },
                           Volume {
                               host_path: SSL_KEY_PATH.to_string(),
                               container_path: SSL_KEY_PATH.to_string(),
                           },
                           Volume {
                               host_path: NGINX_CONF_PATH.to_string(),
                               container_path: NGINX_CONF_PATH.to_string(),
                           }];
        return app;
    }
}

static NGINX_CONF_PATH: &str = "/var/nginx/conf";


fn write_nginx_config_file(content: &str) -> Result<(), io::Error> {
    match File::create(NGINX_CONF_PATH) {
        Ok(mut file) => {
            match file.write_all(content.as_bytes()) {
                Ok(_) => Ok(()),
                Err(e) => Err(e),
            }
        }
        Err(e) => Err(e),
    }
}

#[derive(Deserialize)]
struct ProxyPath {
    path: String,
    app_name: String,
    websocket: bool,
}

pub fn set_proxy(p: &Proxy) -> Result<(), io::Error> {
    match p.build_nginx_conf() {
        Ok(config) => {
            match write_nginx_config_file(config.as_ref()) {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
            let app = p.build_app();
            match containers::set_app(app) {
                OkErr::Ok => Ok(()),
                OkErr::Err(e) => Err(io::Error::new(io::ErrorKind::Other, e.as_ref())),
            }
        }
        Err(e) => return Err(e),
    }
}




static SSL_CERT_PATH: &str = "/etc/ssl/certs/plunk.cert";
static SSL_KEY_PATH: &str = "/etc/ssl/certs/plunk.key";



static SSL_DIRECTIVE: &str = "
        ssl_certificate     /etc/ssl/certs/plunk.crt;
        ssl_certificate_key /etc/ssl/certs/plunk.key;
        ssl_protocols       TLSv1.2;
";


static SSL_CACHE_DIRECTIVE: &str = "
     ssl_session_cache   shared:SSL:10m;
    ssl_session_timeout 10m;
    ";



static WS_PROXY_DIRECTIVE: &str = "
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection \"upgrade\";
";