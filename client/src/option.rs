use std::env;
use client;

extern crate tokio_core;
extern crate hyper;


pub fn run() {
    let mut args: Vec<String> = vec![];
    for argument in env::args() {
        args.push(argument);
    }
    if args.len() < 3 {
        panic!("Too few args");
    }
    match args[1].as_ref() {
        "set_app" => {
            client::set_app(args[2].as_ref());
        }
        _ => {
            panic!("Unknown command");
        }
    }
}
