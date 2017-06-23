mod app;

#[macro_use]  
extern crate serde_derive;  
extern crate iron;
 

fn main() {
    println!("Hello!");
    let mut r = app::Router::new();
    r.register_routes();
    iron::Iron::new(r.routes).http("localhost:3000").unwrap();
    println!("Buy");
}   
  



   