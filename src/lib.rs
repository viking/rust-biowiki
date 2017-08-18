extern crate hyper;
extern crate futures;
extern crate serde;
extern crate serde_json;
#[macro_use] extern crate serde_derive;
extern crate regex;
#[macro_use] extern crate lazy_static;

mod web;
mod page;
mod router;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use hyper::StatusCode;
use hyper::header::{AccessControlAllowOrigin};
use hyper::server::{Http, Request, Response, Service};
use futures::{Future, Stream, BoxFuture};
use web::*;
use page::*;
use router::Route;

struct BioWiki {
    webs: Arc<Mutex<Webs>>
}

impl Service for BioWiki {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = BoxFuture<Self::Response, Self::Error>;

    fn call(&self, request: Request) -> Self::Future {
        let mut response = Response::new().
            with_header(AccessControlAllowOrigin::Any);

        let route = Route::from(&request);
        match route {
            Route::ListWebs => {
                let webs = self.webs.lock().unwrap();
                match webs.list_webs() {
                    Ok(stubs) => {
                        response.set_body(serde_json::to_string(&stubs).unwrap());
                    },
                    Err(_) => {
                        response.set_status(StatusCode::InternalServerError);
                    }
                }
                futures::future::ok(response).boxed()
            },
            Route::CreateWeb => {
                let webs = self.webs.clone();
                request.body().concat2().map(move |body| {
                    let data = body.to_vec();
                    let stub = WebStub::parse(&data);
                    if stub.is_err() {
                        response.set_status(StatusCode::BadRequest);
                        return response;
                    }

                    let stub = stub.unwrap();
                    match webs.lock().unwrap().create_web(&stub.name) {
                        Ok(_) => (),
                        Err(WebError::OverwriteError) => {
                            response.set_status(StatusCode::BadRequest);
                        },
                        Err(_) => {
                            response.set_status(StatusCode::InternalServerError);
                        }
                    }
                    response
                }).boxed()
            },
            Route::ListPages { web_name } => {
                let webs = self.webs.lock().unwrap();
                let web = webs.get_web(&web_name);
                if web.is_none() {
                    response.set_status(StatusCode::NotFound);
                    return futures::future::ok(response).boxed();
                }

                let web = web.unwrap();
                match web.list_pages() {
                    Ok(stubs) => {
                        response.set_body(serde_json::to_string(&stubs).unwrap());
                    },
                    Err(_) => {
                        response.set_status(StatusCode::InternalServerError);
                    }
                }
                futures::future::ok(response).boxed()
            },
            Route::ShowPage { web_name, page_name } => {
                let webs = self.webs.lock().unwrap();
                let web = webs.get_web(&web_name);
                if web.is_none() {
                    response.set_status(StatusCode::NotFound);
                    return futures::future::ok(response).boxed();
                }

                let web = web.unwrap();
                match web.get_page(&page_name) {
                    Ok(page) => {
                        response.set_body(serde_json::to_string(&page.detail).unwrap());
                    },
                    Err(PageError::NotFound) => {
                        response.set_status(StatusCode::NotFound);
                    },
                    Err(err) => {
                        println!("couldn't get page: {}", err);
                        response.set_status(StatusCode::InternalServerError);
                    }
                }
                futures::future::ok(response).boxed()
            },
            Route::CreatePage { web_name } => {
                let webs = self.webs.lock().unwrap();
                let web = webs.get_web(&web_name);
                if web.is_none() {
                    response.set_status(StatusCode::NotFound);
                    return futures::future::ok(response).boxed();
                }

                let web = web.unwrap();
                request.body().concat2().map(move |body| {
                    let data = body.to_vec();
                    let page_detail = PageDetail::parse(&data);
                    if page_detail.is_err() {
                        response.set_status(StatusCode::BadRequest);
                        return response;
                    }

                    let page_detail = page_detail.unwrap();
                    let page = web.new_page(page_detail);
                    match page.create() {
                        Ok(_) => (),
                        Err(PageError::OverwriteError) => {
                            response.set_status(StatusCode::BadRequest);
                        },
                        Err(_) => {
                            response.set_status(StatusCode::InternalServerError);
                        }
                    }
                    response
                }).boxed()
            },
            Route::UpdatePage { web_name, page_name } => {
                let webs = self.webs.lock().unwrap();
                let web = webs.get_web(&web_name);
                if web.is_none() {
                    response.set_status(StatusCode::NotFound);
                    return futures::future::ok(response).boxed();
                }

                let web = web.unwrap();
                let page = web.get_page(&page_name);
                if page.is_err() {
                    response.set_status(StatusCode::InternalServerError);
                    return futures::future::ok(response).boxed();
                }

                let mut page = page.unwrap();
                request.body().concat2().map(move |body| {
                    let data = body.to_vec();
                    let detail = PageDetail::parse(&data);
                    if detail.is_err() {
                        response.set_status(StatusCode::BadRequest);
                        return response;
                    }

                    let detail = detail.unwrap();
                    if &page_name != &detail.name {
                        response.set_status(StatusCode::BadRequest);
                        return response;
                    }
                    page.detail = detail;

                    match page.update() {
                        Ok(_) => (),
                        Err(PageError::NotFound) => {
                            response.set_status(StatusCode::NotFound);
                        },
                        Err(_) => {
                            response.set_status(StatusCode::InternalServerError);
                        }
                    };
                    response
                }).boxed()
            },
            Route::Invalid => {
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
            Ok(BioWiki { webs: webs.clone() })
        }).unwrap();
    server.run().unwrap();
}
