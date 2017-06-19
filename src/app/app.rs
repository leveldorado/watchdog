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




#[derive(Serialize, Deserialize, Debug)]
pub struct App {
    image: String,
    name: String,
    port: u32,
    #[serde(skip)]
    current_container_name: String,
    #[serde(default="UTC::now")]
    pub started_at: DateTime<UTC>,
}











pub fn read_conf(path: &str, mut j_link: String) -> Result<App, String> {
    let r = File::open(path);
    let mut f;
    match r {
        Ok(v) => f = v,
        Err(e) => return Err(e.description().to_string()),
    }
    let r = f.read_to_string(&mut j_link);
    match r {
        Ok(_) => {}
        Err(e) => return Err(e.description().to_string()),
    }
    let r: Result<App, serde_json::Error> = serde_json::from_str(j_link.as_ref());
    match r {
        Ok(v) => Ok(v),
        Err(e) => Err(e.description().to_string()),
    }
}



impl App {
    pub fn start(&mut self) {
        self.clear_previous_container();
        let vars: Vec<EnvVar> = Default::default();
        let args = self.build_run_cmd_args(vars);
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



#[derive(Debug)]
struct EnvVar {
    name: String,
    value: String,
}
