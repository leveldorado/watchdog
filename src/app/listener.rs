use app;
use std::collections::HashMap;

extern crate iron;
extern crate router;
extern crate bodyparser;

use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
extern crate bus;

use std::sync::{Arc, Mutex};
use iron::prelude::Plugin;
use std::error::Error;
use iron::{IronError, Response, Request, IronResult};
use iron::status;




struct Controller {
    apps: HashMap<String, app::App>,
    config: Vec<app::Config>,
    dead: bus::Bus<bool>,
    config_stop_receiver: Receiver<configStop>,
    config_stop_sender: Sender<configStop>,
    config_receiver: Receiver<app::Config>,
    config_sender: Sender<app::Config>,
}


#[derive(Clone)]
enum configStop {
    Some(app::Config),
    Stop,
    Request,
}


impl Controller {
    fn new() -> Self {
        let (config_stop_sender, config_stop_receiver): (Sender<configStop>,
                                                         Receiver<configStop>) = mpsc::channel();
        let (config_sender, config_receiver): (Sender<app::Config>, Receiver<app::Config>) =
            mpsc::channel();
        Controller {
            apps: HashMap::new(),
            config: vec![],
            dead: bus::Bus::new(1),
            config_stop_receiver: config_stop_receiver,
            config_stop_sender: config_stop_sender,
            config_receiver: config_receiver,
            config_sender: config_sender,
        }
    }

    fn handle_config(&mut self, done_sender: &Sender<Done>) {
        match self.config_stop_receiver.recv() {
            Ok(data) => {}
            Err(e) => {
                println!("{}", e);
                return;
            }
        }
    }

    fn handle_app_update(&mut self,
                         config_rec: &Receiver<app::AppStop>,
                         done_sender: &Sender<Done>) {
    }
}


pub struct Router {
    contr: Controller,
    pub routes: router::Router,
}



enum Done {
    Ok,
    Err(String),
}


impl Router {
    pub fn new() -> Self {
        Router {
            routes: router::Router::new(),
            contr: Controller::new(),
        }
    }
    fn register_config_handler(&mut self) {
        let (done_sender, done_receiver): (Sender<Done>, Receiver<Done>) = mpsc::channel();
        let sender = Arc::new(Mutex::new(self.contr.config_stop_sender.clone())).clone();
        let done_receiver = Arc::new(Mutex::new(done_receiver)).clone();
        self.routes
            .post("/config",
                  move |req: &mut Request| -> IronResult<Response> {
                      if let parseAndSendResult::Err(resp) = parse_and_send_config(&sender, req) {
                          return resp;
                      }
                      return check_done_receiver(&done_receiver);

                  },
                  "config");
        self.contr.handle_config(&done_sender);
    }

    fn register_app_handler(&mut self) {
        let (s, r): (Sender<app::AppStop>, Receiver<app::AppStop>) = mpsc::channel();
        let (done_sender, done_receiver): (Sender<Done>, Receiver<Done>) = mpsc::channel();
        self.contr.handle_app_update(&r, &done_sender);
        self.routes
            .post("/app",
                  move |r: &mut iron::Request| -> iron::IronResult<iron::Response> {

                      return Ok(iron::Response::with(iron::status::Ok));
                  },
                  "app");
    }

    pub fn register_routes(&mut self) {
        self.register_config_handler();
        self.register_app_handler();
    }
}


fn check_done_receiver(done_receiver: &Arc<Mutex<Receiver<Done>>>) -> IronResult<Response> {
    match done_receiver.lock() {
        Ok(r) => {
            match r.recv() {
                Ok(done) => {
                    match done {
                        Done::Ok => return Ok(Response::with(status::Ok)),
                        Done::Err(e) => return Ok(Response::with((status::InternalServerError, e))),
                    }
                }
                Err(e) => return Err(IronError::new(e, status::BadRequest)),
            }
        }
        Err(e) => return Ok(Response::with((status::InternalServerError, e.description()))),
    }
}

enum parseAndSendResult {
    Err(IronResult<Response>),
    Ok,
}

fn parse_and_send_config(sender: &Arc<Mutex<Sender<configStop>>>,
                         req: &mut Request)
                         -> parseAndSendResult {
    match req.get::<bodyparser::Struct<app::Config>>() {
        Ok(Some(body)) => {
            match sender.lock() {
                Ok(s) => {
                    if let Err(e) = s.send(configStop::Some(body)) {
                        return parseAndSendResult::Err(Err(IronError::new(e, status::InternalServerError)));
                    }
                }
                Err(e) => {
                    return parseAndSendResult::Err(Ok(Response::with((status::InternalServerError, e.description()))));
                }
            }
        }
        Ok(None) => return parseAndSendResult::Err(Ok(iron::Response::with(status::BadRequest))),
        Err(e) => return parseAndSendResult::Err(Err(IronError::new(e, status::BadRequest))),
    }
    return parseAndSendResult::Ok;
}
