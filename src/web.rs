use std::{io, error, fmt};
use std::path::PathBuf;
use std::convert::From;
use std::fs::{self, File};
use serde_json;

#[derive(Debug)]
pub enum PageReadError {
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

#[derive(Debug)]
pub enum PageWriteError {
    IoError(io::Error),
    JsonError(serde_json::error::Error)
}

impl error::Error for PageWriteError {
    fn description(&self) -> &str {
        match self {
            &PageWriteError::IoError(ref err) => err.description(),
            &PageWriteError::JsonError(ref err) => err.description()
        }
    }
}

impl fmt::Display for PageWriteError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            &PageWriteError::IoError(ref err) => write!(f, "PageWriteError::IoError({})", err),
            &PageWriteError::JsonError(ref err) => write!(f, "PageWriteError::JsonError({})", err),
        }
    }
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
pub struct Page {
    pub name: String,
    content: String
}

#[derive(Serialize)]
pub struct PageStub {
    name: String
}

#[derive(Debug)]
pub struct Web {
    pub name: String,
    pub path: PathBuf
}

impl Web {
    pub fn get_page_stubs(&self) -> Result<Vec<PageStub>, WebReadError> {
        let stubs = fs::read_dir(&self.path)?.filter(|entry| {
            match entry {
                &Err(_) => false,
                &Ok(ref entry) => {
                    let path = entry.path();
                    if !path.is_file() {
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

    pub fn get_page(&self, name: &str) -> Result<Page, PageReadError> {
        let mut path = self.path.clone();
        path.push(name);

        let file = File::open(path)?;
        let page = serde_json::from_reader(file)?;
        Ok(page)
    }

    pub fn save_page(&self, page: Page) -> Result<(), PageWriteError> {
        let mut path = self.path.clone();
        path.push(&page.name);

        let file = File::create(path)?;
        serde_json::to_writer_pretty(file, &page)?;
        Ok(())
    }
}

#[derive(Serialize)]
pub struct WebStub {
    name: String
}

pub type WebReadError = io::Error;

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

    pub fn get_web_stubs(&self) -> Result<Vec<WebStub>, WebReadError> {
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

    pub fn create_web(&self, name: &str) -> Result<Web, io::Error> {
        let mut path = self.path.clone();
        path.push(name);
        fs::create_dir(&path)?;
        Ok(Web { name: name.to_string(), path: path })
    }
}
