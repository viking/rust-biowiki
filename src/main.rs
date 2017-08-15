extern crate hyper;
extern crate futures;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use hyper::{Method, StatusCode, Body, Chunk};
use hyper::header::Location;
use hyper::server::{Http, Request, Response, Service};
use futures::{Future, Stream, BoxFuture};

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Page {
    name: String,
    content: String
}

#[derive(Debug)]
struct Web {
    name: String,
    pages: Mutex<HashMap<String, Page>>
}

impl Web {
    fn new(name: String) -> Web {
        Web { name: name, pages: Mutex::new(HashMap::new()) }
    }

    fn get_page(&self, name: &str) -> Option<Page> {
        match self.pages.lock().unwrap().get(name) {
            None => None,
            Some(page) => Some(page.clone())
        }
    }

    fn put_page(&self, name: String, page: Page) {
        self.pages.lock().unwrap().insert(name, page);
    }
}

type Webs = Arc<Mutex<HashMap<String, Web>>>;

struct Biowiki {
    webs: Webs
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
                let web = webs.get(parts[2]);
                if let None = web {
                    response.set_status(StatusCode::NotFound);
                    return futures::future::ok(response).boxed();
                }
                let web = web.unwrap();

                let page = web.get_page(parts[3]);
                if let None = page {
                    response.set_status(StatusCode::NotFound);
                    return futures::future::ok(response).boxed();
                }
                let page = page.unwrap();

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
                    let mut webs = webs.lock().unwrap();
                    let mut web =
                        if webs.contains_key(&parts[2]) {
                            webs.get_mut(&parts[2]).unwrap()
                        } else {
                            let name = parts[2].to_string();
                            webs.insert(name.clone(), Web::new(name.clone()));
                            webs.get_mut(&name).unwrap()
                        };

                    let name = &parts[3];
                    let data = body.to_vec();
                    match serde_json::from_slice::<Page>(&data) {
                        Ok(page) => {
                            if &page.name != name {
                                response.set_status(StatusCode::BadRequest);
                            } else {
                                web.put_page(name.to_string(), page);
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

fn main() {
    let addr = "127.0.0.1:3000".parse().unwrap();
    let webs = Arc::new(Mutex::new(HashMap::new()));
    let server = Http::new().bind(&addr, move || Ok(Biowiki { webs: webs.clone() })).unwrap();
    server.run().unwrap();
}
