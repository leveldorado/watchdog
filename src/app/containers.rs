use std::sync::Mutex;
use std::collections::HashMap;
use app::{App, Res};
use app::config::OkErr;
use std::error::Error;


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