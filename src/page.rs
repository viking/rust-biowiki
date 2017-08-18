use std::{io, error, fmt};
use std::convert::From;
use std::path::PathBuf;
use std::fs::{self, File};
use serde_json;

const PAGE_FILENAME: &'static str = "page.json";

#[derive(Debug)]
pub enum PageError {
    NotFound,
    NotDirectory,
    InvalidPath,
    Utf8Error,
    NameMismatch,
    IoError(io::Error),
    JsonError(serde_json::error::Error),
    OverwriteError
}

impl error::Error for PageError {
    fn description(&self) -> &str {
        match self {
            &PageError::NotFound => "page not found",
            &PageError::NotDirectory => "page path not a directory",
            &PageError::InvalidPath => "page path is not valid",
            &PageError::NameMismatch => "page detail name does not match",
            &PageError::Utf8Error => "page name is not a valid utf8 string",
            &PageError::IoError(ref err) => err.description(),
            &PageError::JsonError(ref err) => err.description(),
            &PageError::OverwriteError => "page already exists",
        }
    }
}

impl fmt::Display for PageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            &PageError::NotFound => write!(f, "PageError::NotFound"),
            &PageError::NotDirectory => write!(f, "PageError::NotDirectory"),
            &PageError::InvalidPath => write!(f, "PageError::InvalidPath"),
            &PageError::NameMismatch => write!(f, "PageError::NameMismatch"),
            &PageError::Utf8Error => write!(f, "PageError::Utf8Error"),
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
pub struct PageDetail {
    pub name: String,
    pub title: String,
    content: String
}

impl PageDetail {
    pub fn parse(data: &[u8]) -> Result<PageDetail, PageError> {
        let detail = serde_json::from_slice::<PageDetail>(data)?;
        Ok(detail)
    }
}

#[derive(Clone, Debug)]
pub struct Page {
    pub path: PathBuf,
    pub detail: PageDetail
}

impl Page {
    pub fn open(path: PathBuf) -> Result<Page, PageError> {
        if !path.exists() {
            return Err(PageError::NotFound);
        }
        if !path.is_dir() {
            return Err(PageError::NotDirectory);
        }
        let expected_name = {
            let file_name = path.file_name();
            if file_name.is_none() {
                return Err(PageError::InvalidPath);
            }
            let file_name = file_name.unwrap();
            match file_name.to_str() {
                None => return Err(PageError::Utf8Error),
                Some(s) => s.to_string()
            }
        };

        let mut detail_path = path.clone();
        detail_path.push(PAGE_FILENAME);
        let file = File::open(&detail_path)?;
        let detail: PageDetail = serde_json::from_reader(file)?;
        if &detail.name != &expected_name {
            return Err(PageError::NameMismatch);
        }
        Ok(Page { path, detail })
    }

    pub fn create(&self) -> Result<(), PageError> {
        if self.path.exists() {
            return Err(PageError::OverwriteError);
        }
        fs::create_dir(&self.path)?;
        self.write()
    }

    pub fn update(&self) -> Result<(), PageError> {
        if !self.path.exists() {
            return Err(PageError::NotFound);
        }
        self.write()
    }

    fn write(&self) -> Result<(), PageError> {
        let mut detail_path = self.path.clone();
        detail_path.push(PAGE_FILENAME);

        let file = File::create(detail_path)?;
        serde_json::to_writer_pretty(file, &self.detail)?;
        Ok(())
    }
}

#[derive(Serialize)]
pub struct PageStub {
    pub name: String
}
