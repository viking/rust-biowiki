use std::{io, error, fmt};
use std::path::PathBuf;
use std::convert::From;
use std::fs;
use serde_json;

use page::*;

#[derive(Debug)]
pub enum WebError {
    NotFound,
    IoError(io::Error),
    JsonError(serde_json::error::Error),
    OverwriteError
}

impl From<serde_json::error::Error> for WebError {
    fn from(err: serde_json::error::Error) -> WebError {
        WebError::JsonError(err)
    }
}

impl error::Error for WebError {
    fn description(&self) -> &str {
        match self {
            &WebError::NotFound => "web directory not found",
            &WebError::IoError(ref err) => err.description(),
            &WebError::JsonError(ref err) => err.description(),
            &WebError::OverwriteError => "web directory already exists",
        }
    }
}

impl fmt::Display for WebError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            &WebError::NotFound => write!(f, "WebError::NotFound"),
            &WebError::IoError(ref err) => write!(f, "WebError::IoError({})", err),
            &WebError::JsonError(ref err) => write!(f, "WebError::JsonError({})", err),
            &WebError::OverwriteError => write!(f, "WebError::OverwriteError"),
        }
    }
}

impl From<io::Error> for WebError {
    fn from(err: io::Error) -> WebError {
        match err.kind() {
            io::ErrorKind::NotFound => WebError::NotFound,
            _ => WebError::IoError(err)
        }
    }
}

#[derive(Debug)]
pub struct Web {
    pub name: String,
    pub path: PathBuf
}

impl Web {
    pub fn list_pages(&self) -> Result<Vec<PageStub>, WebError> {
        let stubs = fs::read_dir(&self.path)?.filter(|entry| {
            match entry {
                &Err(_) => false,
                &Ok(ref entry) => {
                    let path = entry.path();
                    if !path.is_dir() {
                        return false;
                    }
                    let s = path.to_str();
                    s.is_some()
                }
            }
        }).map(|entry| {
            let name = entry.unwrap().path().file_name().unwrap().to_str().unwrap().to_string();
            PageStub { name }
        }).collect();
        Ok(stubs)
    }

    pub fn get_page(&self, name: &str) -> Result<Page, PageError> {
        let mut path = self.path.clone();
        path.push(name);
        Page::open(path)
    }

    pub fn new_page(&self, detail: PageDetail) -> Page {
        let mut path = self.path.clone();
        path.push(&detail.name);
        Page { path, detail }
    }
}

#[derive(Serialize, Deserialize)]
pub struct WebStub {
    pub name: String
}

impl WebStub {
    pub fn parse(data: &[u8]) -> Result<WebStub, WebError> {
        let stub = serde_json::from_slice::<WebStub>(&data)?;
        Ok(stub)
    }
}

pub struct Webs {
    pub path: PathBuf
}

impl Webs {
    pub fn get_web(&self, name: &str) -> Option<Web> {
        let mut path = self.path.clone();
        path.push(name);
        if path.is_dir() {
            Some(Web { name: name.to_string(), path: path })
        } else {
            None
        }
    }

    pub fn list_webs(&self) -> Result<Vec<WebStub>, WebError> {
        let stubs = fs::read_dir(&self.path)?.filter(|entry| {
            match entry {
                &Err(_) => false,
                &Ok(ref entry) => {
                    let path = entry.path();
                    if !path.is_dir() {
                        return false;
                    }
                    let s = path.to_str();
                    s.is_some()
                }
            }
        }).map(|entry| {
            let name = entry.unwrap().path().file_name().unwrap().to_str().unwrap().to_string();
            WebStub { name }
        }).collect();
        Ok(stubs)
    }

    pub fn create_web(&self, name: &str) -> Result<Web, WebError> {
        let mut path = self.path.clone();
        path.push(name);
        if path.exists() {
            Err(WebError::OverwriteError)
        } else {
            fs::create_dir(&path)?;
            Ok(Web { name: name.to_string(), path: path })
        }
    }
}
