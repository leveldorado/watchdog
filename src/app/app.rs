extern crate serde_json;
extern crate serde;
extern crate chrono;
extern crate uuid;
use std::fs::File;
use std::io::Read;
use std::error::Error;
use self::chrono::prelude::*;
use std::process::{Command, Stdio};
use self::uuid::Uuid;
use std::thread;
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug)]
enum HealthCheckType {
    TCP,
    HTTP,
}


#[derive(Serialize, Deserialize, Debug)]
pub struct App {
    image: String,
    name: String,
    port: u32,
    #[serde(skip, default="UTC::now")]
    last_health_check: DateTime<UTC>,
    health_check_type: HealthCheckType,
    health_check_interval: Duration,
    #[serde(skip)]
    current_container_name: String,
    #[serde(default="UTC::now")]
    pub started_at: DateTime<UTC>,
}



pub fn read_conf<T>(path: &str) -> Result<T, String>
    where T: for<'de> serde::Deserialize<'de>
{
    let r = File::open(path);
    let f;
    match r {
        Ok(v) => f = v,
        Err(e) => return Err(e.description().to_string()),
    }
    let r: Result<T, serde_json::Error> = serde_json::from_reader(f);
    match r {
        Ok(v) => Ok(v),
        Err(e) => Err(e.description().to_string()),
    }
}



impl App {
    pub fn restart(&mut self, ask_config: &Sender<bool>, config_chan: &Receiver<Config>) {
        self.clear_previous_container();
        self.start(ask_config, config_chan);
    }
}

impl App {
    pub fn start(&mut self, ask_config: &Sender<bool>, config_chan: &Receiver<Config>) {
        ask_config.send(true).unwrap();
        let config = config_chan.recv().unwrap();
        let args = self.build_run_cmd_args(config.vars);
        self.do_command(args);
        self.started_at = UTC::now();
    }
}

impl App {
    fn clear_previous_container(&mut self) {
        if self.current_container_name.len() == 0 {
            return;
        }
        let args: Vec<String> = vec!["stop".to_string(), self.current_container_name.clone()];
        match self.do_command(args) {
            Res::Ok => {}
            Res::Err(e) => println!("{}", e),
        }
        let args: Vec<String> = vec!["rm".to_string(), self.current_container_name.clone()];
        match self.do_command(args) {
            Res::Ok => {
                self.current_container_name = String::new();
            }
            Res::Err(e) => println!("{}", e),
        }
    }
}

enum Res {
    Ok,
    Err(String),
}

impl App {
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
                return Res::Ok;
            }
            Err(e) => {
                return Res::Err(format!("{:?}", e));
            }
        }
    }
}


const WATCHDOG_PREFIX: &str = "watchdog";

impl App {
    fn build_run_cmd_args(&mut self, vars: Vec<EnvVar>) -> Vec<String> {
        let mut cmd = vec!["run".to_string(), "--name".to_string()];
        let name: String = format!("{}_{}_{}",
                                   WATCHDOG_PREFIX,
                                   self.name,
                                   Uuid::new_v4().to_string());
        self.current_container_name.push_str(name.as_ref());
        cmd.push(self.current_container_name.to_string());
        cmd.push("-p".to_string());
        cmd.push(format!("{}:{}", self.port, self.port));
        cmd.push("-d".to_string());
        cmd.push(format!("{}", self.image));
        for var in vars {
            cmd.push(format!("-e {}={}", var.name, var.value))
        }
        return cmd;
    }
}


#[derive(Debug, Clone, Deserialize)]
struct EnvVar {
    name: String,
    value: String,
}



#[derive(Default, Clone, Deserialize)]
pub struct Config {
    hash_sum: String,
    vars: Vec<EnvVar>,
}



pub fn run() {
    let (app_sender, app_receiver): (Sender<AppStop>, Receiver<AppStop>) = mpsc::channel();
    thread::spawn(move || match read_conf("/home/vasyl/rust/watchdog/examples/config/app.json") {
                      Ok(app) => app_sender.send(AppStop::App(app)).unwrap(),
                      Err(e) => {
                          println!("{}", e);
                          app_sender.send(AppStop::Stop).unwrap();
                      }
                  });
    let (checked_app_sender, checked_app_receiver): (Sender<App>, Receiver<App>) = mpsc::channel();
    let (config_ask_sender, config_ask_receiver): (Sender<bool>, Receiver<bool>) = mpsc::channel();
    let (config_sender, config_receiver): (Sender<Config>, Receiver<Config>) = mpsc::channel();
    handle_app(&app_receiver,
               &checked_app_sender,
               &config_ask_sender,
               &config_receiver);
}


pub enum AppStop {
    App(App),
    Stop,
}

fn handle_app(r: &Receiver<AppStop>,
              s: &Sender<App>,
              ask_config: &Sender<bool>,
              config_chan: &Receiver<Config>) {
    loop {
        let mut app: App;
        match r.recv().unwrap() {
            AppStop::Stop => return,
            AppStop::App(a) => app = a,
        }
        let last_health_check_interval = UTC::now()
            .signed_duration_since(app.last_health_check)
            .to_std()
            .unwrap();
        if app.current_container_name.len() == 0 {
            app.start(ask_config, config_chan);
        } else if last_health_check_interval > app.health_check_interval {
            if !app.check_health() {
                app.restart(ask_config, config_chan);
            }
        }
        let app = app;
        s.send(app).unwrap();
    }
}


impl App {
    fn check_health(&mut self) -> bool {
        return true;
    }
}
