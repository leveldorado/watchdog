mod app;

#[macro_use]  
extern crate serde_derive;  
 

fn main() {
    println!("Hello, world!");
    let  r = app::read_conf("/home/vasyl/rust/watchdog/examples/config/app.json");
    match r {
        Ok(v) =>  println!("{:?}", v),
        Err(e) => println!("{}", e),
    }
   
}




   