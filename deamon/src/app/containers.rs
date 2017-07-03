use std::sync::Mutex;
use std::collections::HashMap;
use app::{App, Res, HealthCheckRes, MemoryCheckRes};
use app::config::OkErr;
use std::error::Error;
use std::time::Duration;
use std::thread::sleep;
extern crate futures;
extern crate hyper;
extern crate tokio_core;

use self::tokio_core::reactor::Core;
use self::hyper::client::HttpConnector;
use std::io;


lazy_static! {
    static ref CONTAINERS: Mutex<HashMap<String, App>> = Mutex::new(HashMap::new());
}

#[derive(Debug)]
struct ContainersDropout {}

impl Drop for ContainersDropout {
    fn drop(&mut self) {
        match CONTAINERS.lock() {
            Ok(m) => {
                for (name, app) in m.iter() {
                    println!("Container: {} drop; \n {:?}", name, app.clear());
                }
            }
            Err(e) => println!("{}", e),
        }

    }
}


pub fn set_app(a: App) -> OkErr {
    let mut a: App = a;
    a.health_check_interval = Duration::new(a.health_check_interval_in_seconds as u64, 0);
    if let Err(e) = a.convert_string_memory_limit_to_kilobytes() {
        return OkErr::Err(e);
    }
    match CONTAINERS.lock() {
        Ok(mut m) => {
            match m.get(&a.name) {
                Some(old_app) => {
                    if let OkErr::Err(e) = update_app(&old_app, &mut a) {
                        return OkErr::Err(e);
                    }
                }
                None => {
                    if let OkErr::Err(e) = add_app(&mut a, true) {
                        return OkErr::Err(e);
                    }
                }
            }
            m.insert(a.name.clone(), a);
        }
        Err(e) => return OkErr::Err(e.description().to_string()),
    }

    return OkErr::Ok;
}


fn update_app(old: &App, a: &mut App) -> OkErr {
    if let Res::Err(e) = old.clear() {
        return OkErr::Err(e);
    }
    return add_app(a, false);
}


fn add_app(a: &mut App, need_to_clear: bool) -> OkErr {
    if need_to_clear {
        a.clear_by_name();
    }
    match a.start() {
        Res::Ok(_) => OkErr::Ok,
        Res::Err(e) => OkErr::Err(e),
    }
}

pub fn get_app(name: String) -> Result<App, io::Error> {
    match CONTAINERS.lock() {
        Ok(c) => {
            let name: String = name.clone();
            match c.get(&name) {
                Some(app) => Ok(app.clone()),
                None => return Err(io::Error::new(io::ErrorKind::NotFound, "Not found")),
            }
        }
        Err(e) => Err(io::Error::new(io::ErrorKind::Other, e.description())),
    }
}


pub fn remove_app(name: &str) {
    match CONTAINERS.lock() {
        Ok(mut c) => {
            match c.remove(name) {
                Some(app) => {
                    println!("{}\n{:?}", app.name, app.clear());
                }
                None => {}
            }
        }
        Err(e) => println!("{:?}", e),
    }
}


fn watch_interval() -> Duration {
    return Duration::new(1, 0);
}

pub fn watch() {
    let interval = watch_interval();
    let drop = ContainersDropout {};
    println!("{:?}", &drop);
    loop {
        iter();
        sleep(interval);
    }
}

fn iter() {
    let client: hyper::Client<HttpConnector, hyper::Body>;
    match Core::new() {
        Ok(c) => {
            client = hyper::Client::new(&c.handle());
        }
        Err(e) => {
            print!("{}", e);
            return;
        }
    }
    match CONTAINERS.lock() {
        Ok(mut containers) => {
            let mut checked_containers: Vec<App> = vec![];
            for (_, old) in containers.iter() {
                let mut app = old.clone();
                match app.health_check(&client) {
                    HealthCheckRes::Ok => {}
                    HealthCheckRes::UnHealth => {
                        if let OkErr::Err(e) = app.restart("UNHEALTH") {
                            println!("{}", e);
                        }
                    }
                    HealthCheckRes::Err(e) => print!("{}", e),
                }
                match app.memory_check() {
                    MemoryCheckRes::Ok => {}
                    MemoryCheckRes::Exceed(memory) => {
                        if let OkErr::Err(e) = app.restart(format!("Exceed {}", memory).as_ref()) {
                            println!("{}", e);
                        }
                    }
                    MemoryCheckRes::Err(e) => print!("{}", e),
                }
                checked_containers.push(app);
            }
            for app in checked_containers {
                containers.insert(app.name.clone(), app);
            }
        }
        Err(e) => print!("{}", e),
    }
}
