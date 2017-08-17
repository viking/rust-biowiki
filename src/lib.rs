extern crate hyper;
extern crate futures;
extern crate serde;
extern crate serde_json;
#[macro_use] extern crate serde_derive;
extern crate regex;
#[macro_use] extern crate lazy_static;

mod web;
mod router;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use hyper::StatusCode;
use hyper::server::{Http, Request, Response, Service};
use futures::{Future, Stream, BoxFuture};
use web::*;
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
        let mut response = Response::new();

        let route = Route::from(&request);
        match route {
            Route::ListWebs => {
                let webs = self.webs.lock().unwrap();
                match webs.get_web_stubs() {
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
                    match serde_json::from_slice::<WebStub>(&data) {
                        Ok(stub) => {
                            match webs.lock().unwrap().create_web(&stub.name) {
                                Ok(_) => (),
                                Err(WebError::OverwriteError) => {
                                    response.set_status(StatusCode::BadRequest);
                                },
                                Err(_) => {
                                    response.set_status(StatusCode::InternalServerError);
                                }
                            }
                        },
                        Err(_) => {
                            response.set_status(StatusCode::BadRequest);
                        }
                    }
                    response
                }).boxed()
            },
            Route::ListPages { web_name } => {
                let webs = self.webs.lock().unwrap();
                match webs.get_web(&web_name) {
                    None => {
                        response.set_status(StatusCode::NotFound);
                    },
                    Some(web) => {
                        match web.get_page_stubs() {
                            Ok(stubs) => {
                                response.set_body(serde_json::to_string(&stubs).unwrap());
                            },
                            Err(_) => {
                                response.set_status(StatusCode::InternalServerError);
                            }
                        }
                    }
                }
                futures::future::ok(response).boxed()
            },
            Route::ShowPage { web_name, page_name } => {
                let webs = self.webs.lock().unwrap();
                match webs.get_web(&web_name) {
                    None => {
                        response.set_status(StatusCode::NotFound);
                    },
                    Some(web) => {
                        match web.get_page(&page_name) {
                            Ok(page) => {
                                response.set_body(serde_json::to_string(&page).unwrap());
                            },
                            Err(PageError::NotFound) => {
                                response.set_status(StatusCode::NotFound);
                            },
                            Err(_) => {
                                response.set_status(StatusCode::InternalServerError);
                            }
                        }
                    }
                }
                futures::future::ok(response).boxed()
            },
            Route::CreatePage { web_name } => {
                let webs = self.webs.lock().unwrap();
                match webs.get_web(&web_name) {
                    None => {
                        response.set_status(StatusCode::NotFound);
                        futures::future::ok(response).boxed()
                    },
                    Some(web) => {
                        request.body().concat2().map(move |body| {
                            let data = body.to_vec();
                            match serde_json::from_slice::<Page>(&data) {
                                Ok(page) => {
                                    match web.create_page(page) {
                                        Ok(_) => (),
                                        Err(PageError::OverwriteError) => {
                                            response.set_status(StatusCode::BadRequest);
                                        },
                                        Err(_) => {
                                            response.set_status(StatusCode::InternalServerError);
                                        }
                                    }
                                },
                                Err(_) => {
                                    response.set_status(StatusCode::BadRequest);
                                }
                            }
                            response
                        }).boxed()
                    }
                }
            },
            Route::UpdatePage { web_name, page_name } => {
                let webs = self.webs.lock().unwrap();
                match webs.get_web(&web_name) {
                    None => {
                        response.set_status(StatusCode::NotFound);
                        futures::future::ok(response).boxed()
                    },
                    Some(web) => {
                        request.body().concat2().map(move |body| {
                            let data = body.to_vec();
                            match serde_json::from_slice::<Page>(&data) {
                                Ok(page) => {
                                    if &page_name != &page.name {
                                        response.set_status(StatusCode::BadRequest);
                                        return response;
                                    }
                                    match web.update_page(page) {
                                        Ok(_) => (),
                                        Err(PageError::NotFound) => {
                                            response.set_status(StatusCode::NotFound);
                                        },
                                        Err(_) => {
                                            response.set_status(StatusCode::InternalServerError);
                                        }
                                    }
                                },
                                Err(_) => {
                                    response.set_status(StatusCode::BadRequest);
                                }
                            }
                            response
                        }).boxed()
                    }
                }
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
