use std::{error, fmt};
use std::io::{self, Read, Write};
use std::convert::From;
use std::path::PathBuf;
use std::fs::{self, File};
use serde_json;
use base64;
use mime::{self, Mime};
use regex::Regex;

const PAGE_FILENAME: &'static str = "page.json";
const ATTACHMENTS_DIRECTORY: &'static str = "attachments";

#[derive(Debug)]
pub enum AttachmentError {
    NotFound,
    IoError(io::Error),
    JsonError(serde_json::error::Error),
    Base64Error(base64::DecodeError),
}

impl error::Error for AttachmentError {
    fn description(&self) -> &str {
        match self {
            &AttachmentError::NotFound => "page not found",
            &AttachmentError::IoError(ref err) => err.description(),
            &AttachmentError::JsonError(ref err) => err.description(),
            &AttachmentError::Base64Error(ref err) => err.description(),
        }
    }
}

impl fmt::Display for AttachmentError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            &AttachmentError::NotFound => write!(f, "AttachmentError::NotFound"),
            &AttachmentError::IoError(ref err) => write!(f, "AttachmentError::IoError({})", err),
            &AttachmentError::JsonError(ref err) => write!(f, "AttachmentError::JsonError({})", err),
            &AttachmentError::Base64Error(ref err) => write!(f, "AttachmentError::Base64Error({})", err),
        }
    }
}

impl From<io::Error> for AttachmentError {
    fn from(err: io::Error) -> AttachmentError {
        match err.kind() {
            io::ErrorKind::NotFound => AttachmentError::NotFound,
            _ => AttachmentError::IoError(err)
        }
    }
}

impl From<serde_json::error::Error> for AttachmentError {
    fn from(err: serde_json::error::Error) -> AttachmentError {
        AttachmentError::JsonError(err)
    }
}

impl From<base64::DecodeError> for AttachmentError {
    fn from(err: base64::DecodeError) -> AttachmentError {
        AttachmentError::Base64Error(err)
    }
}

pub struct Attachment {
    pub path: PathBuf
}

impl Attachment {
    pub fn open(path: PathBuf) -> Result<Attachment, AttachmentError> {
        if !path.exists() {
            return Err(AttachmentError::NotFound);
        }
        Ok(Attachment { path })
    }

    pub fn data(&self) -> Result<Vec<u8>, AttachmentError> {
        let mut file = File::open(&self.path)?;
        let mut buf = Vec::new();
        let _ = file.read_to_end(&mut buf)?;
        Ok(buf)
    }

    pub fn mime_type(&self) -> Mime {
        lazy_static! {
            static ref PNG_RE: Regex = Regex::new(r"^(?i)png$").unwrap();
        }

        let ext = self.path.extension();
        if ext.is_none() {
            return mime::APPLICATION_OCTET_STREAM;
        }

        let ext = ext.unwrap().to_str();
        if ext.is_none() {
            return mime::APPLICATION_OCTET_STREAM;
        }

        let ext = ext.unwrap();
        if PNG_RE.is_match(ext) {
            mime::IMAGE_PNG
        } else {
            mime::APPLICATION_OCTET_STREAM
        }
    }
}

#[derive(Deserialize)]
pub struct AttachmentData {
    pub file_name: String,
    pub encoded_data: String
}

impl AttachmentData {
    pub fn parse(data: &[u8]) -> Result<AttachmentData, AttachmentError> {
        let att_data = serde_json::from_slice::<AttachmentData>(data)?;
        Ok(att_data)
    }

    pub fn data(&self) -> Result<Vec<u8>, AttachmentError> {
        let data = base64::decode(&self.encoded_data)?;
        Ok(data)
    }

    pub fn is_file_name_valid(&self) -> bool {
        lazy_static! {
            static ref FILE_NAME_RE: Regex = Regex::new(r"^\w+\.\w+$").unwrap();
        }
        FILE_NAME_RE.is_match(&self.file_name)
    }
}

#[derive(Serialize)]
pub struct AttachmentStub {
    file_name: String
}

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

    fn write(&self) -> Result<(), PageError> {
        let mut detail_path = self.path.clone();
        detail_path.push(PAGE_FILENAME);
        let detail_file = File::create(detail_path)?;
        serde_json::to_writer_pretty(detail_file, &self.detail)?;
        Ok(())
    }

    pub fn list_attachments(&self) -> Result<Vec<AttachmentStub>, AttachmentError> {
        let mut path = self.path.clone();
        path.push(ATTACHMENTS_DIRECTORY);
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
}

#[derive(Serialize)]
pub struct PageStub {
    pub name: String
}
