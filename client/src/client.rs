use std::fs::File;
use std::io::Read;
extern crate hyper;


static DEAMON_ADDR: &str = "http://localhost:3000/";

pub fn set_app(path: &str) {
    let mut file = File::open(path).unwrap();
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();
    let cl = hyper::Client::new();
    let mut resp = cl.post(&build_url("app")).body(&content).send().unwrap();
    if resp.status == hyper::Ok {
        println!("DONE");
    } else {
        let mut resp_body = String::new();
        resp.read_to_string(&mut resp_body).unwrap();
        println!("{}{}", resp.status, resp_body);
    }
}



fn build_url(path: &str) -> String {
    return  format!("{}{}", DEAMON_ADDR, path);   
}