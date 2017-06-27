use std::sync::Mutex;
use std::collections::HashMap;
use app::{App, Res, HealthCheckRes};
use app::config::OkErr;
use std::error::Error;
use std::time::Duration;
use std::thread::sleep;
extern crate futures;
extern crate hyper;
extern crate tokio_core;

use self::tokio_core::reactor::Core;
use self::hyper::client::HttpConnector;


lazy_static! {
    static ref CONTAINERS: Mutex<HashMap<String, App>> = Mutex::new(HashMap::new());
}





pub fn set_app(a: App) -> OkErr {
    let mut a: App = a;
    match CONTAINERS.lock() {
        Ok(mut m) => {
            match m.get(&a.name) {
                Some(old_app) => {
                    if let OkErr::Err(e) = update_app(&old_app, &mut a) {
                        return OkErr::Err(e);
                    }
                }
                None => {
                    if let OkErr::Err(e) = add_app(&mut a) {
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
    return add_app(a);
}


fn add_app(a: &mut App) -> OkErr {
    match a.start() {
        Res::Ok(_) => OkErr::Ok,
        Res::Err(e) => OkErr::Err(e),
    }
}


fn watch_interval() -> Duration {
    return Duration::new(1, 0);
}

pub fn watch() {
    let interval = watch_interval();
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
                    HealthCheckRes::UnHealth => {}
                    HealthCheckRes::Err(e) => print!("{}", e),
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
