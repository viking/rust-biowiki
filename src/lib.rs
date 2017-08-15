extern crate hyper;
extern crate futures;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

mod web;

use std::{io, fs};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use hyper::{Method, StatusCode};
use hyper::header::Location;
use hyper::server::{Http, Request, Response, Service};
use futures::{Future, Stream, BoxFuture};
use web::{Page, Web, PageReadError};

struct Webs {
    path: PathBuf
}

impl Webs {
    fn get_web(&self, name: &str) -> Option<Web> {
        let mut path = self.path.clone();
        path.push(name);
        if path.is_dir() {
            Some(Web { name: name.to_string(), path: path })
        } else {
            None
        }
    }

    fn create_web(&self, name: &str) -> Result<Web, io::Error> {
        let mut path = self.path.clone();
        path.push(name);
        fs::create_dir(&path)?;
        Ok(Web { name: name.to_string(), path: path })
    }
}

struct Biowiki {
    webs: Arc<Mutex<Webs>>
}

impl Service for Biowiki {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = BoxFuture<Self::Response, Self::Error>;

    fn call(&self, request: Request) -> Self::Future {
        let mut response = Response::new();

        let method = request.method().clone();
        let path = request.path().to_string();
        match (method, path) {
            (Method::Get, path) => {
                if path == "/" {
                    response.headers_mut().set(Location::new("/wiki/Home/WebHome"));
                    response.set_body("{}");
                    return futures::future::ok(response).boxed()
                }

                let parts: Vec<_> = path.split('/').collect();
                if parts.len() != 4 {
                    response.set_status(StatusCode::NotFound);
                    return futures::future::ok(response).boxed();
                }
                if parts[1] != "wiki" {
                    response.set_status(StatusCode::NotFound);
                    return futures::future::ok(response).boxed();
                }

                let webs = self.webs.lock().unwrap();
                let web = webs.get_web(parts[2]);
                if let None = web {
                    response.set_status(StatusCode::NotFound);
                    return futures::future::ok(response).boxed();
                }
                let web = web.unwrap();

                let page = match web.get_page(parts[3]) {
                    Ok(page) => page,
                    Err(PageReadError::NotFound) => {
                        response.set_status(StatusCode::NotFound);
                        return futures::future::ok(response).boxed();
                    },
                    Err(err) => {
                        response.set_status(StatusCode::InternalServerError);
                        return futures::future::ok(response).boxed();
                    }
                };

                response.set_body(serde_json::to_string(&page).unwrap());
                futures::future::ok(response).boxed()
            },
            (Method::Post, path) => {
                let parts: Vec<_> = path.split('/').map(|s| s.to_string()).collect();
                if parts.len() != 4 {
                    response.set_status(StatusCode::NotFound);
                    return futures::future::ok(response).boxed();
                }
                if parts[1] != "wiki" {
                    response.set_status(StatusCode::NotFound);
                    return futures::future::ok(response).boxed();
                }
                let webs = self.webs.clone();
                request.body().concat2().map(move |body| {
                    let webs = webs.lock().unwrap();
                    let web = match webs.get_web(&parts[2]) {
                        Some(web) => web,
                        None => {
                            match webs.create_web(&parts[2]) {
                                Ok(web) => web,
                                Err(err) => {
                                    response.set_status(StatusCode::InternalServerError);
                                    return response;
                                }
                            }
                        }
                    };

                    let name = &parts[3];
                    let data = body.to_vec();
                    match serde_json::from_slice::<Page>(&data) {
                        Ok(page) => {
                            if &page.name != name {
                                response.set_status(StatusCode::BadRequest);
                            } else {
                                match web.save_page(page) {
                                    Ok(_) => (),
                                    Err(err) => {
                                        response.set_status(StatusCode::InternalServerError);
                                    }
                                }
                            }
                        },
                        Err(err) => {
                            response.set_status(StatusCode::BadRequest);
                        }
                    }
                    response
                }).boxed()
            },
            _ => {
                response.set_status(StatusCode::NotFound);
                futures::future::ok(response).boxed()
            },
        }
    }
}

pub fn run(host: String, port: String, path: PathBuf) {
    let addr = format!("{}:{}", host, port).parse().unwrap();
    let webs = Arc::new(Mutex::new(Webs { path: path }));
    let server =
        Http::new().bind(&addr, move || {
            Ok(Biowiki { webs: webs.clone() })
        }).unwrap();
    server.run().unwrap();
}
