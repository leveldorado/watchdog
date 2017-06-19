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
    #[serde(skip)]
    previous_container_name: String,
    #[serde(default="UTC::now")]
    pub started_at: DateTime<UTC>,        
}


   








pub fn read_conf(path: &str) -> Result<App, String> {
    let r = File::open(path);
    let mut f;
    match r {
        Ok(v) => f = v,
        Err(e) => return Err(e.description().to_string()),
    }
    let mut j = String::new();
    let r = f.read_to_string(&mut j);
    match r {
        Ok(_) => {} ,
        Err(e) => return Err(e.description().to_string()),
    }
    let r: Result<App, serde_json::Error> = serde_json::from_str(&j);
    match r {
        Ok(v) => Ok(v),
        Err(e) => Err(e.description().to_string()),
    }
}



impl App {
    pub fn start(&mut self) {
        let args = self.build_run_cmd_args();
        let cmd = Command::new("docker").args(args).
        stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().expect("work");
        let mut s = String::new(); 
        match cmd.stdout.unwrap().read_to_string(&mut s) {
            Ok(_) => println!("{}", s),
            Err(e) => println!("{:?}", e),
        }
    }
}


const WATCHDOG_PREFIX: &str = "watchdog";   

impl App {
    fn build_run_cmd_args(&mut self) -> Vec<String> {
        let mut cmd = vec!["run".to_string(), "--name".to_string()];
        let  container_name = format!("{}_{}_{}", WATCHDOG_PREFIX, self.name, Uuid::new_v4().to_string());
        cmd.push(container_name);
        cmd.push("-p".to_string());
        let port = format!("{}:{}", self.port, self.port);
        cmd.push(port);
        cmd.push("-d".to_string());
        let image = format!("{}", self.image);
        cmd.push(image);
        return cmd;
    }
}

   

