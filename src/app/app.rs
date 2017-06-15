extern crate serde_json;
extern crate serde; 
extern crate chrono;
use std::fs::File;
use std::io::Read;
use std::error::Error;
use self::chrono::prelude::*;




#[derive(Serialize, Deserialize, Debug)]
pub struct App {
    image: String,
    name: String, 
    port: u32,
    currentContainerName: String,
    previousContainerName: String,
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

pub fn track(app: &mut App) {
     
}


pub fn start(app: &mut App) {
    app.currentContainerName = app.name + "_" + app.name;
    let cmd = "docker run --name " + app.currentContainerName ;

}


