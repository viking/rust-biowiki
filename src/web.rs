use std::{io, error, fmt};
use std::path::PathBuf;
use std::convert::From;
use std::fs::{self, File};
use serde_json;

#[derive(Debug)]
pub enum PageError {
    NotFound,
    IoError(io::Error),
    JsonError(serde_json::error::Error),
    OverwriteError
}

impl error::Error for PageError {
    fn description(&self) -> &str {
        match self {
            &PageError::NotFound => "page file not found",
            &PageError::IoError(ref err) => err.description(),
            &PageError::JsonError(ref err) => err.description(),
            &PageError::OverwriteError => "page file already exists",
        }
    }
}

impl fmt::Display for PageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            &PageError::NotFound => write!(f, "PageError::NotFound"),
            &PageError::IoError(ref err) => write!(f, "PageError::IoError({})", err),
            &PageError::JsonError(ref err) => write!(f, "PageError::JsonError({})", err),
            &PageError::OverwriteError => write!(f, "PageError::OverwriteError"),
        }
    }
}

impl From<io::Error> for PageError {
    fn from(err: io::Error) -> PageError {
        match err.kind() {
            io::ErrorKind::NotFound => PageError::NotFound,
            _ => PageError::IoError(err)
        }
    }
}

impl From<serde_json::error::Error> for PageError {
    fn from(err: serde_json::error::Error) -> PageError {
        PageError::JsonError(err)
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Page {
    pub name: String,
    content: String
}

impl Page {
    pub fn parse(data: &Vec<u8>) -> Result<Page, PageError> {
        let page = serde_json::from_slice::<Page>(data)?;
        Ok(page)
    }
}

#[derive(Serialize)]
pub struct PageStub {
    name: String
}

#[derive(Debug)]
pub enum WebError {
    NotFound,
    IoError(io::Error),
    OverwriteError
}

impl error::Error for WebError {
    fn description(&self) -> &str {
        match self {
            &WebError::NotFound => "web directory not found",
            &WebError::IoError(ref err) => err.description(),
            &WebError::OverwriteError => "web directory already exists",
        }
    }
}

impl fmt::Display for WebError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            &WebError::NotFound => write!(f, "WebError::NotFound"),
            &WebError::IoError(ref err) => write!(f, "WebError::IoError({})", err),
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
    pub fn get_page_stubs(&self) -> Result<Vec<PageStub>, WebError> {
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

    pub fn get_page(&self, name: &str) -> Result<Page, PageError> {
        let mut path = self.path.clone();
        path.push(name);

        let file = File::open(path)?;
        let page = serde_json::from_reader(file)?;
        Ok(page)
    }

    pub fn create_page(&self, page: Page) -> Result<(), PageError> {
        let mut path = self.path.clone();
        path.push(&page.name);

        if path.exists() {
            Err(PageError::OverwriteError)
        } else {
            let file = File::create(path)?;
            serde_json::to_writer_pretty(file, &page)?;
            Ok(())
        }
    }

    pub fn update_page(&self, page: Page) -> Result<(), PageError> {
        let mut path = self.path.clone();
        path.push(&page.name);

        if !path.exists() {
            Err(PageError::NotFound)
        } else {
            let file = File::create(path)?;
            serde_json::to_writer_pretty(file, &page)?;
            Ok(())
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct WebStub {
    pub name: String
}

impl WebStub {
    pub fn parse(data: &Vec<u8>) -> Result<Page, PageError> {
        let page = serde_json::from_slice::<Page>(data)?;
        Ok(page)
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

    pub fn get_web_stubs(&self) -> Result<Vec<WebStub>, WebError> {
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
