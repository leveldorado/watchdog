mod app;

#[macro_use]  
extern crate serde_derive;  
extern crate iron;
#[macro_use]
extern crate lazy_static;
extern crate router;   

use std::thread;
 

fn main() {
    println!("Hello!");
    let mut r = router::Router::new();
    app::register_routes(&mut r);

    thread::spawn(app::watch);
    iron::Iron::new(r).http("localhost:3000").unwrap();
    println!("Buy");
}   
  



   