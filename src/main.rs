extern crate hyper;
extern crate futures;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate getopts;

use std::{env, io, error, fmt};
use std::path::PathBuf;
use std::convert::From;
use std::fs::{self, File};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use getopts::Options;
use hyper::{Method, StatusCode, Body, Chunk};
use hyper::header::Location;
use hyper::server::{Http, Request, Response, Service};
use futures::{Future, Stream, BoxFuture};

#[derive(Debug)]
enum PageReadError {
    NotFound,
    IoError(io::Error),
    JsonError(serde_json::error::Error)
}

impl error::Error for PageReadError {
    fn description(&self) -> &str {
        match self {
            &PageReadError::NotFound => "page file not found",
            &PageReadError::IoError(ref err) => err.description(),
            &PageReadError::JsonError(ref err) => err.description()
        }
    }
}

impl fmt::Display for PageReadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            &PageReadError::NotFound => write!(f, "PageReadError::NotFound"),
            &PageReadError::IoError(ref err) => write!(f, "PageReadError::IoError({})", err),
            &PageReadError::JsonError(ref err) => write!(f, "PageReadError::JsonError({})", err),
        }
    }
}

impl From<io::Error> for PageReadError {
    fn from(err: io::Error) -> PageReadError {
        match err.kind() {
            io::ErrorKind::NotFound => PageReadError::NotFound,
            _ => PageReadError::IoError(err)
        }
    }
}

impl From<serde_json::error::Error> for PageReadError {
    fn from(err: serde_json::error::Error) -> PageReadError {
        PageReadError::JsonError(err)
    }
}

enum PageWriteError {
    IoError(io::Error),
    JsonError(serde_json::error::Error)
}

impl From<io::Error> for PageWriteError {
    fn from(err: io::Error) -> PageWriteError {
        PageWriteError::IoError(err)
    }
}

impl From<serde_json::error::Error> for PageWriteError {
    fn from(err: serde_json::error::Error) -> PageWriteError {
        PageWriteError::JsonError(err)
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Page {
    name: String,
    content: String
}

#[derive(Debug)]
struct Web {
    name: String,
    path: PathBuf
}

impl Web {
    fn get_page(&self, name: &str) -> Result<Page, PageReadError> {
        let mut path = self.path.clone();
        path.push(name);

        let file = File::open(path)?;
        let page = serde_json::from_reader(file)?;
        Ok(page)
    }

    fn save_page(&self, page: Page) -> Result<(), PageWriteError> {
        let mut path = self.path.clone();
        path.push(&page.name);

        let file = File::create(path)?;
        serde_json::to_writer_pretty(file, &page)?;
        Ok(())
    }
}

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

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optopt("h", "host", "listen on host (default: localhost)", "HOST");
    opts.optopt("p", "port", "listen on port (default: 3000)", "PORT");
    opts.reqopt("d", "dir", "directory for wiki files", "PATH");
    opts.optflag("", "help", "print this help menu");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            println!("{}", f);
            print_usage(&program, opts);
            return;
        }
    };
    if matches.opt_present("help") {
        print_usage(&program, opts);
        return;
    }

    let host =
        if matches.opt_present("h") {
            matches.opt_str("h").unwrap()
        } else {
            "127.0.0.1".to_string()
        };

    let port =
        if matches.opt_present("p") {
            matches.opt_str("p").unwrap()
        } else {
            "3000".to_string()
        };

    let dir = matches.opt_str("d").unwrap();
    let path = PathBuf::from(dir);
    if !path.is_dir() {
        println!("{} is not a directory", path.to_str().unwrap());
        return;
    }

    let addr = format!("{}:{}", host, port).parse().unwrap();
    let webs = Arc::new(Mutex::new(Webs { path: path }));
    let server =
        Http::new().bind(&addr, move || {
            Ok(Biowiki { webs: webs.clone() })
        }).unwrap();
    server.run().unwrap();
}
