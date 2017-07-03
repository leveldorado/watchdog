use std::fs::File;
use std::io::Read;
extern crate hyper;


static DEAMON_ADDR: &str = "http://localhost:3000/";

pub fn set(path: &str, object_type: &str) {
    let mut file = File::open(path).unwrap();
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();
    let cl = hyper::Client::new();
    let mut resp = cl.post(&build_url(object_type)).body(&content).send().unwrap();
    if resp.status == hyper::Ok {
        println!("DONE");
    } else {
        let mut resp_body = String::new();
        resp.read_to_string(&mut resp_body).unwrap();
        println!("{}{}", resp.status, resp_body);
    }
}


pub fn get_app(name: String) {
    do_get(format!("app?app={}", name).as_ref())
}


pub fn get_config() {
    do_get("config")
}

fn do_get(path: &str) {
    let cl = hyper::Client::new();
    let mut resp = cl.get(&build_url(path)).send().unwrap();
    let mut resp_body = String::new();
    resp.read_to_string(&mut resp_body).unwrap();
    println!("{}{}", resp.status, resp_body);
}


pub fn rm_app(name: String) {
    let cl = hyper::Client::new();
    let mut resp = cl.request(hyper::method::Method::Delete, &build_url(format!("app?app={}", name).as_ref())).send().unwrap();
    let mut resp_body = String::new();
    resp.read_to_string(&mut resp_body).unwrap();
    println!("{}{}", resp.status, resp_body);
}




fn build_url(path: &str) -> String {
    return  format!("{}{}", DEAMON_ADDR, path);   
}