use std::sync::Mutex;
use std::error::Error;

lazy_static! {
    static ref CONFIG: Mutex<Config> = Mutex::new(Config::new());
}


#[derive(Deserialize, Serialize, Clone)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
}


#[derive(Default,  Clone, Deserialize)]
pub struct Config {
    pub vars: Vec<EnvVar>,
}


impl Config {
    fn new() -> Config {
        return Config { vars: vec![] };
    }

    fn set_config(&mut self, vars: Vec<EnvVar>) {
        self.vars = vars;
    }
}


pub enum OkErr {
    Ok,
    Err(String),
}

pub fn set_vars(vars: Vec<EnvVar>) -> OkErr {
    match CONFIG.lock() {
        Ok(mut c) => c.set_config(vars),
        Err(e) => return OkErr::Err(e.description().to_string()),
    }
    return OkErr::Ok;
}

pub enum GetVarsRes {
    Vars(Vec<EnvVar>),
    Err(String),
}

pub fn get_vars() -> GetVarsRes {
    match CONFIG.lock() {
        Ok(c) => GetVarsRes::Vars(c.vars.clone()),
        Err(e) => return GetVarsRes::Err(e.description().to_string()),
    }
}
