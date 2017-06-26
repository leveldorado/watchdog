extern crate iron;
extern crate router;
extern crate bodyparser;

use iron::prelude::Plugin;
use std::error::Error;
use iron::{IronError, Response, Request, IronResult};
use iron::status;
use app::config::{Config, OkErr};  
use app::config; 
use app::containers;
use app::App;

extern crate serde_json;   





enum ParseConfigRes {
    Err(IronResult<Response>),
    Ok(Config),
}

fn parse_config(req: &mut Request)
                         -> ParseConfigRes {
    match req.get::<bodyparser::Struct<Config>>() {
        Ok(Some(body)) => {return ParseConfigRes::Ok(body)},
        Ok(None) => return ParseConfigRes::Err(Ok(iron::Response::with(status::BadRequest))),
        Err(e) => return ParseConfigRes::Err(Err(IronError::new(e, status::BadRequest))),
    }
}


enum ParseAppRes {
    Err(IronResult<Response>),
    Ok(App),
}

fn parse_app(req: &mut Request)
                         -> ParseAppRes {
    match req.get::<bodyparser::Struct<App>>() {
        Ok(Some(body)) => {return ParseAppRes::Ok(body)},
        Ok(None) => return ParseAppRes::Err(Ok(iron::Response::with(status::BadRequest))),
        Err(e) => return ParseAppRes::Err(Err(IronError::new(e, status::BadRequest))),
    }
}



pub fn register_routes(r: &mut router::Router) {
        register_config_routes(r);
        register_containers_routes(r);
}


fn register_config_routes(r: &mut router::Router) {
      r.post("/config",
                  move |req: &mut Request| -> IronResult<Response> {
                      match parse_config(req) {
                          ParseConfigRes::Ok(c) => {
                                match config::set_vars(c.vars) {
                                    OkErr::Ok => return Ok(Response::with(status::Ok)),
                                    OkErr::Err(e) => return Ok(Response::with((status::InternalServerError, e))),
                                }
                          },  
                          ParseConfigRes::Err(resp) => {
                              return resp;
                          }
                      }
                  },
                  "set_config");
      r.get("/config",
                  move |_: &mut Request| -> IronResult<Response> {
                      match config::get_vars() {
                          config::GetVarsRes::Vars(vars) => {
                               match serde_json::to_string(&vars) {
                                   Ok(data) => return Ok(Response::with((status::Ok, data))),
                                   Err(e) => return Ok(Response::with((status::InternalServerError, e.description()))),
                               }
                          },  
                          config::GetVarsRes::Err(e) => {
                              return Ok(Response::with((status::InternalServerError, e)));
                          }
                      }
                  },
                  "get_config");            
}



fn register_containers_routes(r: &mut router::Router) {
      r.post("/app",
                  move |req: &mut Request| -> IronResult<Response> {
                      match parse_app(req) {
                          ParseAppRes::Ok(app) => {
                                match containers::set_app(app) {
                                    OkErr::Ok => return Ok(Response::with(status::Ok)),
                                    OkErr::Err(e) => return Ok(Response::with((status::InternalServerError, e))),
                                }
                          },  
                          ParseAppRes::Err(resp) => {
                              return resp;
                          }
                      }
                  },
                  "set_app");           
}