extern crate serde_json;
extern crate serde;
extern crate chrono;
use std::io::Read;
use self::chrono::prelude::*;
use std::process::{Command, Stdio};
use std::time::Duration;
use app::config;
use std::error::Error;

extern crate futures;
extern crate hyper;
extern crate tokio_core;

use self::futures::Future;
use self::hyper::client::HttpConnector;
use std::str::FromStr;




#[derive(Serialize, Deserialize, Debug, Clone)]
enum HealthCheckType {
    TCP,
    HTTP,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct App {
    #[serde(skip)]
    id: String,
    image: String,
    pub name: String,
    port: u32,
    #[serde(skip, default="UTC::now")]
    last_health_check: DateTime<UTC>,
    health_check_type: HealthCheckType,
    #[serde(default="String::new")]
    health_path: String,
    health_check_interval: Duration,
    unhealth_threshould: u32,
    health_threshould: u32,
    #[serde(skip)]
    unhealth_count: u32,
    health_count: u32,
    #[serde(default="Default::default")]
    volume: Volume,
    #[serde(default="UTC::now")]
    pub started_at: DateTime<UTC>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct Volume {
    host_path: String,
    container_path: String,
}


impl Volume {
    fn get_v_command(&self) -> String {
        return format!("{}:{}", self.host_path, self.container_path);
    }
}



pub enum HealthCheckRes {
    Ok,
    UnHealth,
    Err(String),
}


impl App {
    pub fn health_check(&mut self,
                        cl: &hyper::Client<HttpConnector, hyper::Body>)
                        -> HealthCheckRes {
        let last_health_check_interval: Duration;
        match UTC::now()
                  .signed_duration_since(self.last_health_check)
                  .to_std() {
            Ok(d) => last_health_check_interval = d,
            Err(e) => return HealthCheckRes::Err(e.description().to_string()),
        }
        if last_health_check_interval < self.health_check_interval {
            return HealthCheckRes::Ok;
        }
        let health: HealthCheckRes;
        match self.health_check_type {
            HealthCheckType::HTTP => health = self.check_http_health(cl),
            HealthCheckType::TCP => health = self.check_tcp_health(),
        }
        self.last_health_check = UTC::now();
        match health {
            HealthCheckRes::Ok => {
                if self.unhealth_count > 0 {
                    self.health_count = self.health_count + 1;
                    if self.health_count >= self.health_threshould {
                        self.unhealth_count = 0;
                    }
                }
                return HealthCheckRes::Ok;
            }
            HealthCheckRes::Err(_) => return health,
            HealthCheckRes::UnHealth => {
                self.health_count = 0;
                self.unhealth_count = self.unhealth_count + 1;
                if self.unhealth_count >= self.unhealth_threshould {
                    return health;
                }
                return HealthCheckRes::Ok;
            }
        }
    }
    fn check_http_health(&self,
                         cl: &hyper::Client<hyper::client::HttpConnector, hyper::Body>)
                         -> HealthCheckRes {
        let url: hyper::Uri;
        match get_url(self.port, &self.health_path) {
            URIRes::Ok(u) => url = u,
            URIRes::Err(e) => return HealthCheckRes::Err(e),
        }
        match cl.get(url).wait() {
            Ok(resp) => {
                if resp.status() == hyper::Ok {
                    return HealthCheckRes::Ok;
                } else {
                    return HealthCheckRes::UnHealth;
                }
            }
            Err(e) => return HealthCheckRes::Err(e.description().to_string()),
        }
    }

    fn check_tcp_health(&self) -> HealthCheckRes {
        return HealthCheckRes::Ok;
    }
    pub fn start(&mut self) -> Res {
        let variables: Vec<config::EnvVar>;
        match config::get_vars() {
            config::GetVarsRes::Vars(vars) => variables = vars,
            config::GetVarsRes::Err(e) => return Res::Err(e),
        }
        let mut args = self.build_run_cmd_args(&variables);
        if let Res::Err(e) = self.do_command(args) {
            return Res::Err(e);
        }
        args = vec!["ps".to_string(),
                    "-aqf".to_string(),
                    format!("\"name={}\"", self.name)];
        match self.do_command(args) {
            Res::Ok(id) => {
                self.id = id;
                self.started_at = UTC::now();
                return Res::Ok(String::new());
            }
            Res::Err(e) => return Res::Err(e),
        }
    }
    pub fn clear(&self) -> Res {
        let mut args: Vec<String> = vec!["stop".to_string(), self.id.clone()];
        if let Res::Err(e) = self.do_command(args) {
            return Res::Err(e);
        }
        args = vec!["rm".to_string(), self.id.clone()];
        return self.do_command(args);
    }
    fn do_command(&self, args: Vec<String>) -> Res {
        let cmd = Command::new("docker")
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("work");
        let mut s = String::new();
        match cmd.stdout.unwrap().read_to_string(&mut s) {
            Ok(_) => {
                return Res::Ok(s);
            }
            Err(e) => {
                return Res::Err(format!("{:?}", e));
            }
        }
    }
    fn build_run_cmd_args(&mut self, vars: &Vec<config::EnvVar>) -> Vec<String> {
        let mut cmd = vec!["run".to_string(), "--name".to_string()];
        cmd.push(self.name.clone());
        cmd.push("-p".to_string());
        cmd.push(format!("{}:{}", self.port, self.port));
        cmd.push("-d".to_string());
        cmd.push(format!("{}", self.image));
        for var in vars {
            cmd.push(format!("-e {}={}", var.name, var.value))
        }
        if self.volume.host_path.len() != 0 {
            cmd.push("-v".to_string());
            cmd.push(self.volume.get_v_command());
        }
        return cmd;
    }
}



pub enum Res {
    Ok(String),
    Err(String),
}

enum URIRes {
    Ok(hyper::Uri),
    Err(String),
}

fn get_url(port: u32, path: &String) -> URIRes {
    let raw_url = format!("http://localhost:{}{}", port, path);
    match hyper::Uri::from_str(raw_url.as_ref()) {
        Ok(u) => return URIRes::Ok(u),
        Err(e) => return URIRes::Err(e.description().to_string()),
    }
}
