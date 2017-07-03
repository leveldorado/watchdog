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
use app::proxy;
use app::App;

extern crate serde_json;
extern crate serde;
extern crate urlencoded;
use self::urlencoded::UrlEncodedQuery;
use std::io::Read;


use self::serde::de;





enum ParseConfigRes {
    Err(IronResult<Response>),
    Ok(Config),
}

fn parse_config(req: &mut Request) -> ParseConfigRes {
    match req.get::<bodyparser::Struct<Config>>() {
        Ok(Some(body)) => return ParseConfigRes::Ok(body),
        Ok(None) => return ParseConfigRes::Err(Ok(iron::Response::with(status::BadRequest))),
        Err(e) => return ParseConfigRes::Err(Err(IronError::new(e, status::BadRequest))),
    }
}


enum ParseAppRes {
    Err(IronResult<Response>),
    Ok,
}

fn parse<T>(body: &mut iron::request::Body, ob: &mut T) -> ParseAppRes
    where for<'a> T: de::Deserialize<'a>
{
    let r: Result<T, serde_json::Error> = serde_json::from_reader(body);
    match r {
        Ok(app) => {
            *ob = app;
            return ParseAppRes::Ok;
        }
        Err(e) => ParseAppRes::Err(Err(IronError::new(e, status::BadRequest))),
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
            }
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
                    Err(e) => {
                        return Ok(Response::with((status::InternalServerError, e.description())))
                    }
                }
            }
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
        let mut app: App = App::new();
        match parse(req.body.by_ref(), &mut app) {
            ParseAppRes::Ok => {
                match containers::set_app(app) {
                    OkErr::Ok => return Ok(Response::with(status::Ok)),
                    OkErr::Err(e) => return Ok(Response::with((status::InternalServerError, e))),
                }
            }
            ParseAppRes::Err(resp) => {
                return resp;
            }
        }
    },
           "set_app");
    r.get("/app",
          move |req: &mut Request| -> IronResult<Response> {
        match req.get_ref::<UrlEncodedQuery>() {
            Ok(m) => {
                match m.get("app") {
                    Some(name) => {
                        match containers::get_app(name[0].clone()) {
                            Ok(app) => {
                                match serde_json::to_string(&app) {
                                    Ok(data) => Ok(Response::with((status::Ok, data))),
                                    Err(e) => Err(IronError::new(e, status::InternalServerError)),
                                }
                            }
                            Err(e) => Err(IronError::new(e, status::InternalServerError)),
                        }
                    }
                    None => Ok(Response::with(status::BadRequest)),
                }
            }
            Err(e) => Err(IronError::new(e, status::InternalServerError)),
        }
    },
          "get_app");


    r.delete("/app",
             move |req: &mut Request| -> IronResult<Response> {
        match req.get_ref::<UrlEncodedQuery>() {
            Ok(m) => {
                match m.get("app") {
                    Some(name) => {
                        containers::remove_app(name[0].as_ref());
                        return Ok(Response::with(status::Ok));
                    }
                    None => {
                        return Ok(Response::with(status::BadRequest));
                    }
                }
            }
            Err(e) => return Ok(Response::with((status::InternalServerError, e.description()))),
        }
    },
             "delete_app");
    r.post("/proxy",
           move |req: &mut Request| -> IronResult<Response> {
        let mut pr: proxy::Proxy = Default::default();
        match parse(req.body.by_ref(), &mut pr) {
            ParseAppRes::Ok => {
                match proxy::set_proxy(&pr) {
                    Ok(_) => Ok(Response::with(status::Ok)),
                    Err(e) => Err(IronError::new(e, status::InternalServerError)),
                }
            }
            ParseAppRes::Err(resp) => {
                return resp;
            }
        }
    },
           "set_proxy");
}
