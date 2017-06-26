mod app;

#[macro_use]  
extern crate serde_derive;  
extern crate iron;
#[macro_use]
extern crate lazy_static;
extern crate router;
 

fn main() {
    let a = "a".to_string() == "a".to_string();
    println!("Hello! {}", a);
    let mut r = router::Router::new();
    app::register_routes(&mut r);
    iron::Iron::new(r).http("localhost:3000").unwrap();
    println!("Buy");
}   
  



   