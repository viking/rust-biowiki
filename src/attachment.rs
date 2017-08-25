use std::{error, fmt};
use std::io::{self, Read};
use std::convert::From;
use std::path::PathBuf;
use std::fs::File;
use serde_json;
use base64;
use mime::{self, Mime};
use regex::Regex;

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
            static ref JPG_RE: Regex = Regex::new(r"^(?i)jpe?g$").unwrap();
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
        } else if JPG_RE.is_match(ext) {
            mime::IMAGE_JPEG
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
            static ref FILE_NAME_RE: Regex = Regex::new(r"^.+\.\w+$").unwrap();
        }
        FILE_NAME_RE.is_match(&self.file_name)
    }
}

#[derive(Serialize)]
pub struct AttachmentStub {
    pub file_name: String
}
