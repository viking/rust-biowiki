use std::error;
use std::fmt::{self, Write as FmtWrite};
use std::io::{self, Write as IoWrite};
use std::convert::From;
use std::path::PathBuf;
use std::fs::{self, File};
use serde_json;
use sha2::{Sha256};
use digest::{Input, FixedOutput};

use attachment::*;

const PAGE_FILENAME: &'static str = "page.json";
const ATTACHMENTS_DIRECTORY: &'static str = "attachments";
const VERSIONS_DIRECTORY: &'static str = "versions";

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
            &PageError::Utf8Error => "page name is not a valid utf8 string",
            &PageError::NameMismatch => "page detail name does not match",
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
            &PageError::Utf8Error => write!(f, "PageError::Utf8Error"),
            &PageError::NameMismatch => write!(f, "PageError::NameMismatch"),
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
    content: String,
    parent: String
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

        // read page detail file
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
        let detail: PageDetail = {
            let mut detail_path = path.clone();
            detail_path.push(PAGE_FILENAME);
            let detail_file = File::open(&detail_path)?;
            serde_json::from_reader(detail_file)?
        };
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

    fn page_path(&self) -> PathBuf {
        let mut page_path = self.path.clone();
        page_path.push(PAGE_FILENAME);
        page_path
    }

    fn version_path(&self, hash: &str) -> PathBuf {
        let mut version_path = self.path.clone();
        version_path.push(VERSIONS_DIRECTORY);
        let file_name = format!("{}.json", &hash);
        version_path.push(&file_name);
        version_path
    }

    fn write(&self) -> Result<(), PageError> {
        let data = serde_json::to_string_pretty(&self.detail)?;
        let data = data.as_ref();

        // write main file
        {
            let page_path = self.page_path();
            let mut page_file = File::create(page_path)?;
            page_file.write_all(data)?;
        }

        // write version file
        {
            let mut hasher = Sha256::default();
            hasher.process(data);
            let result = hasher.fixed_result();
            let mut hash = String::new();
            for byte in result {
                write!(&mut hash, "{:x}", byte).expect("Unable to write");
            }
            let version_path = self.version_path(&hash);
            {
                let versions_path = version_path.parent().unwrap();
                if !versions_path.exists() {
                    fs::create_dir(versions_path)?;
                }
            }
            if !version_path.exists() {
                let mut version_file = File::create(version_path)?;
                version_file.write_all(data)?;
            }
        }
        Ok(())
    }

    pub fn list_attachments(&self) -> Result<Vec<AttachmentStub>, AttachmentError> {
        let mut path = self.path.clone();
        path.push(ATTACHMENTS_DIRECTORY);
        if !path.exists() {
            return Ok(Vec::new());
        }

        let stubs = fs::read_dir(&path)?.filter(|entry| {
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
            let file_name = entry.unwrap().path().file_name().unwrap().to_str().unwrap().to_string();
            AttachmentStub { file_name }
        }).collect();
        Ok(stubs)
    }

    pub fn get_attachment(&self, file_name: &str) -> Result<Attachment, AttachmentError> {
        let mut path = self.path.clone();
        path.push(ATTACHMENTS_DIRECTORY);
        path.push(file_name);
        Attachment::open(path)
    }

    pub fn save_attachment(&self, att_data: AttachmentData) -> Result<(), AttachmentError> {
        let data = att_data.data()?;

        let mut att_path = self.path.clone();
        att_path.push(ATTACHMENTS_DIRECTORY);
        if !att_path.exists() {
            fs::create_dir(&att_path)?;
        }
        att_path.push(att_data.file_name);

        let mut att_file = File::create(att_path)?;
        att_file.write_all(&data)?;
        Ok(())
    }

    pub fn list_versions(&self) -> Result<Vec<VersionStub>, PageError> {
        let mut path = self.path.clone();
        path.push(VERSIONS_DIRECTORY);
        if !path.exists() {
            return Ok(Vec::new());
        }

        let stubs = fs::read_dir(&path)?.filter(|entry| {
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
            let hash = entry.unwrap().path().file_stem().unwrap().to_str().unwrap().to_string();
            VersionStub { hash }
        }).collect();
        Ok(stubs)
    }

    pub fn get_version(&self, hash: &str) -> Result<PageDetail, PageError> {
        let version_path = self.version_path(hash);
        let version_file = File::open(&version_path)?;
        let detail = serde_json::from_reader(version_file)?;
        Ok(detail)
    }
}

#[derive(Serialize)]
pub struct PageStub {
    pub name: String
}

#[derive(Serialize)]
pub struct VersionStub {
    hash: String
}
