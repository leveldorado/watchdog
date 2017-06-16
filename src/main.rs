mod app;

#[macro_use]  
extern crate serde_derive;  
 

fn main() {
    println!("Hello, world!");
    let mut r = app::read_conf("/home/vasyl/rust/watchdog/examples/config/app.json").expect("UHU");
    r.start()
}




   