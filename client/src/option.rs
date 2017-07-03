use std::env;
use client;

extern crate tokio_core;
extern crate hyper;


pub fn run() {
    let mut args: Vec<String> = vec![];
    for argument in env::args() {
        args.push(argument);
    }
    if args.len() < 2 {
        println!("Too few args");
        return;
    }
    if args[1] != "get_config" && args.len() < 3 {
        println!("Too few args");
        return;
    }
    let second_arg = args[2].clone();
    match args[1].as_ref() {
        "set_app" => {
            client::set(second_arg.as_ref(), "app");
        }
        "get_app" => {
            client::get_app(second_arg);
        }
        "rm_app" => {
            client::rm_app(second_arg);
        }
        "set_config" => {
            client::set(second_arg.as_ref(), "config");
        }
        "get_config" => {
            client::get_config();
        }
        _ => {
            panic!("Unknown command");
        }
    }
}
