extern crate serde_json;
extern crate serde;
extern crate chrono;
use std::io::Read;
use self::chrono::prelude::*;
use std::process::{Command, Stdio};
use std::time::Duration;
use app::config;
use std::error::Error;
use std::iter::FromIterator;

extern crate futures;
extern crate hyper;
extern crate tokio_core;

use self::futures::Future;
use self::hyper::client::HttpConnector;
use std::str::FromStr;
use std::net::TcpStream;
use app::config::OkErr;

extern crate lettre;

use self::lettre::transport::smtp::{SecurityLevel, SmtpTransport, SmtpTransportBuilder};
use self::lettre::email::EmailBuilder;
use self::lettre::transport::EmailTransport;
use self::lettre::transport::smtp::authentication::Mechanism;

use std::env;




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
    max_restarts: u32,
    restarts: u32,
    #[serde(skip)]
    unhealth_count: u32,
    health_count: u32,
    memory_notify_limit: String,
    #[serde(skip)]
    memory_notify_limit_in_kilobytes: u64,
    memory_limit: String,
    #[serde(skip)]
    memory_limit_in_kilobytes: u64,
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

pub enum MemoryCheckRes {
    Ok,
    Exceed(u64),
    Err(String),
}


impl App {
    pub fn convert_string_memory_limit_to_kilobytes(&mut self) -> Result<(), String> {
        self.memory_limit_in_kilobytes = parse_memory_usage_to_kilobytes(self.memory_limit
                                                                             .as_ref());
        self.memory_notify_limit_in_kilobytes =
            parse_memory_usage_to_kilobytes(self.memory_notify_limit.as_ref());
        if self.memory_limit_in_kilobytes < self.memory_notify_limit_in_kilobytes {
            return Err("Memory limit less than notify memory limit".to_string());
        }
        return Ok(());
    }
    pub fn memory_check(&self) -> MemoryCheckRes {
        let args: Vec<&str> = vec!["stats", "--no-stream=true", self.id.as_ref()];
        match self.do_command(args) {
            Res::Ok(output) => {
                let mem_usage = parse_memory_usage(output.as_ref());
                if mem_usage > self.memory_limit_in_kilobytes {
                    return MemoryCheckRes::Exceed(mem_usage);
                }
                if mem_usage > self.memory_notify_limit_in_kilobytes {
                    let message = format!("{} EXCEED MEMORY LIMIT {} at {}",
                                          self.name,
                                          self.memory_notify_limit,
                                          UTC::now());
                    match send_report(message.as_ref(), message.as_ref()) {
                        OkErr::Ok => {}
                        OkErr::Err(e) => println!("{}", e),
                    }
                }
                return MemoryCheckRes::Ok;
            }
            Res::Err(e) => return MemoryCheckRes::Err(e),
        }
    }
    pub fn restart(&mut self, reason: &str) -> OkErr {
        let id = self.id.clone();
        {
            let args: Vec<&str> = vec!["logs", "--tail=500", id.as_ref()];
            match self.do_command(args) {
                Res::Ok(log) => {
                    println!("{}", log);
                    match send_report(format!("RESTART {}  DUE TO {}", UTC::now(), reason)
                                          .as_ref(),
                                      log.as_ref()) {
                        OkErr::Ok => {}
                        OkErr::Err(e) => println!("{}", e),
                    }
                    match self.clear() {
                        Res::Ok(_) => {
                            match self.start() {
                                Res::Ok(_) => {
                                    self.restarts = self.restarts + 1;
                                    return OkErr::Ok;
                                }
                                Res::Err(e) => OkErr::Err(e),
                            }
                        }
                        Res::Err(e) => OkErr::Err(e),
                    }
                }
                Res::Err(e) => OkErr::Err(e),
            }

        }
    }
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
        let url = format!("localhost:{}", self.port);
        match TcpStream::connect(url) {
            Ok(_) => HealthCheckRes::Ok,
            Err(_) => HealthCheckRes::UnHealth,
        }
    }
    pub fn start(&mut self) -> Res {
        let variables: Vec<config::EnvVar>;
        match config::get_vars() {
            config::GetVarsRes::Vars(vars) => variables = vars,
            config::GetVarsRes::Err(e) => return Res::Err(e),
        }

        {
            let args = self.build_run_cmd_args(&variables);
            if let Res::Err(e) = self.do_command(Vec::from_iter(args.iter().map(String::as_str))) {
                return Res::Err(e);
            }
        }
        let name_arg: String = format!("\"name={}\"", self.name.clone());
        let args = vec!["ps", "-aqf", name_arg.as_ref()];
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
        let id = self.id.clone();
        let mut args: Vec<&str> = vec!["stop", id.as_ref()];
        if let Res::Err(e) = self.do_command(args) {
            return Res::Err(e);
        }
        args = vec!["rm", id.as_ref()];
        return self.do_command(args);
    }
    fn do_command(&self, args: Vec<&str>) -> Res {
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
        cmd.push(self.image.clone());
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


#[derive(Clone, Default)]
pub struct EmailSettings {
    host: String,
    port: u16,
    username: String,
    password: String,
    report_email: String,
    from_email: String,
    from_name: String,
}


fn get_email_settings() -> EmailSettings {
    let mut settings: EmailSettings = Default::default();
    match env::var("SMTP_HOST") {
        Ok(smtp_host) => settings.host = smtp_host,
        Err(_) => println!("SMTP_HOST is empty"),
    }
    match env::var("SMTP_PORT") {
        Ok(smtp_port) => {
            match smtp_port.parse::<u16>() {
                Ok(number_port) => settings.port = number_port,
                Err(e) => println!("SMTP_PORT {}", e),
            }
        }
        Err(_) => println!("SMTP_PORT is empty"),
    }
    match env::var("SMTP_USERNAME") {
        Ok(smtp_username) => settings.username = smtp_username,
        Err(_) => println!("SMTP_USERNAME is empty"),
    }
    match env::var("SMTP_PASSWORD") {
        Ok(smtp_password) => settings.password = smtp_password,
        Err(_) => println!("SMTP_PASSWORD is empty"),
    }
    match env::var("REPORT_EMAIL") {
        Ok(report_email) => settings.report_email = report_email,
        Err(_) => println!("REPORT_EMAIL is empty"),
    }
    match env::var("FROM_EMAIL") {
        Ok(from_email) => settings.from_email = from_email,
        Err(_) => println!("FROM_EMAIL is empty"),
    }
    match env::var("FROM_NAME") {
        Ok(from_name) => settings.from_name = from_name,
        Err(_) => println!("FROM_NAME is empty"),
    }
    return settings;
}


fn send_report(subject: &str, text: &str) -> OkErr {
    let conf = get_email_settings();
    let mut tr: SmtpTransport;
    match get_smtp_transport(&conf) {
        Ok(mailer) => tr = mailer,
        Err(e) => return OkErr::Err(e),
    }
    match EmailBuilder::new()
              .to(conf.report_email.as_ref())
              .from(conf.from_email.as_ref())
              .body(text)
              .subject(subject)
              .build() {
        Ok(email) => {
            match tr.send(email) {
                Ok(_) => return OkErr::Ok,
                Err(e) => return OkErr::Err(e.description().to_string()),
            }
        }
        Err(e) => return OkErr::Err(e.description().to_string()),
    }
}



pub fn get_smtp_transport(config: &EmailSettings) -> Result<SmtpTransport, String> {
    match SmtpTransportBuilder::new((config.host.as_ref(), config.port)) {
        Ok(mailer) => {
            return Ok(mailer
                          .hello_name(config.from_name.as_ref())
                          .credentials(config.username.as_ref(), config.password.as_ref())
                          .security_level(SecurityLevel::AlwaysEncrypt)
                          .smtp_utf8(true)
                          .authentication_mechanism(Mechanism::CramMd5)
                          .connection_reuse(true)
                          .build())
        }
        Err(e) => return Err(e.description().to_string()),
    }
}


fn parse_memory_usage(docker_output: &str) -> u64 {
    let parts: Vec<&str> = docker_output.split("\n").collect();
    if parts.len() != 2 {
        return 0;
    }
    let split_index: usize;
    match parts[0].find("MEM USAGE") {
        Some(index) => split_index = index,
        None => return 0,
    }
    if split_index > parts[1].len() {
        return 0;
    }
    let (_, last_part) = parts[1].split_at(split_index);
    match last_part.find("/") {
        Some(index) => {
            let (memory_usage, _) = last_part.split_at(index);
            return parse_memory_usage_to_kilobytes(memory_usage.trim());
        }
        None => {
            return 0;
        }
    }
}


fn parse_memory_usage_to_kilobytes(memory_usage: &str) -> u64 {
    let parts: Vec<&str> = memory_usage.split(" ").collect();
    if parts.len() != 2 {
        return 0;
    }
    let number: u64;
    match parts[0].parse::<u64>() {
        Ok(n) => number = n,
        Err(e) => {
            number = 0;
            print!("{}", e);
        }
    }
    match parts[1] {
        "KiB" => return number,
        "MiB" => return 1000 * number,
        "GiB" => return 1000 * 1000 * number,
        _ => {
            return number;
        }
    }
}
