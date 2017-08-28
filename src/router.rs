use std::collections::HashMap;
use regex::Regex;
use hyper::{Request, Method};

struct ParamPath {
    names: Vec<String>,
    re: Regex
}

impl ParamPath {
    fn new(pattern: &str) -> ParamPath {
        let mut names = Vec::new();
        let mut re = String::from("^");
        for part in pattern.split('/').skip(1) {
            re.push('/');

            let mut chars = part.chars();
            match chars.next() {
                Some(c) => {
                    if c == ':' {
                        let name: String = chars.collect();
                        names.push(name.clone());

                        let part = format!(r"(?P<{}>[^/]+)", name);
                        re.push_str(&part);
                    } else {
                        re.push_str(part);
                    }
                },
                None => re.push_str(part)
            };

        }
        re.push('$');

        ParamPath { names: names, re: Regex::new(&re).unwrap() }
    }

    fn test(&self, path: &str) -> Option<HashMap<String, String>> {
        match self.re.captures(path) {
            Some(caps) => {
                let map = self.names.iter().fold(HashMap::new(), |mut map, name| {
                    if let Some(m) = caps.name(name) {
                        map.insert(name.clone(), m.as_str().to_string());
                    }
                    map
                });
                if map.len() == self.names.len() {
                    Some(map)
                } else {
                    None
                }
            },
            None => None
        }
    }
}

pub enum Route {
    ListWebs,
    CreateWeb,
    ListPages  { web_name: String },
    CreatePage { web_name: String },
    ShowPage   { web_name: String, page_name: String },
    UpdatePage { web_name: String, page_name: String },
    ListAttachments  { web_name: String, page_name: String },
    CreateAttachment { web_name: String, page_name: String },
    ServeAttachment  { web_name: String, page_name: String, attachment_name: String },
    ListPageVersions { web_name: String, page_name: String },
    ShowPageVersion  { web_name: String, page_name: String, version_hash: String },
    Invalid
}

impl<'a> From<&'a Request> for Route {
    fn from(request: &'a Request) -> Route {
        lazy_static! {
            static ref WEBS_PATH: ParamPath        = ParamPath::new("/webs");
            //static ref WEB_PATH:  ParamPath      = ParamPath::new("/webs/:web_name");
            static ref PAGES_PATH: ParamPath       = ParamPath::new("/webs/:web_name/pages");
            static ref PAGE_PATH: ParamPath        = ParamPath::new("/webs/:web_name/pages/:page_name");
            static ref ATTACHMENTS_PATH: ParamPath = ParamPath::new("/webs/:web_name/pages/:page_name/attachments");
            static ref ATTACHMENT_PATH: ParamPath  = ParamPath::new("/webs/:web_name/pages/:page_name/attachments/:attachment_name");
            static ref VERSIONS_PATH: ParamPath    = ParamPath::new("/webs/:web_name/pages/:page_name/versions");
            static ref VERSION_PATH: ParamPath     = ParamPath::new("/webs/:web_name/pages/:page_name/versions/:version_hash");
        }
        let path = request.path();
        match request.method() {
            &Method::Get => {
                if let Some(_) = WEBS_PATH.test(&path) {
                    Route::ListWebs

                } else if let Some(mut params) = PAGES_PATH.test(&path) {
                    Route::ListPages { web_name: params.remove("web_name").unwrap() }

                } else if let Some(mut params) = PAGE_PATH.test(&path) {
                    Route::ShowPage {
                        web_name:  params.remove("web_name").unwrap(),
                        page_name: params.remove("page_name").unwrap()
                    }
                } else if let Some(mut params) = ATTACHMENTS_PATH.test(&path) {
                    Route::ListAttachments {
                        web_name:  params.remove("web_name").unwrap(),
                        page_name: params.remove("page_name").unwrap()
                    }
                } else if let Some(mut params) = ATTACHMENT_PATH.test(&path) {
                    Route::ServeAttachment {
                        web_name:  params.remove("web_name").unwrap(),
                        page_name: params.remove("page_name").unwrap(),
                        attachment_name: params.remove("attachment_name").unwrap()
                    }
                } else if let Some(mut params) = VERSIONS_PATH.test(&path) {
                    Route::ListPageVersions {
                        web_name:  params.remove("web_name").unwrap(),
                        page_name: params.remove("page_name").unwrap()
                    }
                } else if let Some(mut params) = VERSION_PATH.test(&path) {
                    Route::ShowPageVersion {
                        web_name:  params.remove("web_name").unwrap(),
                        page_name: params.remove("page_name").unwrap(),
                        version_hash: params.remove("version_hash").unwrap()
                    }
                } else {
                    Route::Invalid
                }
            },
            &Method::Post => {
                if let Some(_) = WEBS_PATH.test(&path) {
                    Route::CreateWeb

                } else if let Some(mut params) = PAGES_PATH.test(&path) {
                    Route::CreatePage { web_name: params.remove("web_name").unwrap() }

                } else if let Some(mut params) = ATTACHMENTS_PATH.test(&path) {
                    Route::CreateAttachment {
                        web_name:  params.remove("web_name").unwrap(),
                        page_name: params.remove("page_name").unwrap()
                    }
                } else {
                    Route::Invalid
                }
            },
            &Method::Put => {
                if let Some(mut params) = PAGE_PATH.test(&path) {
                    Route::UpdatePage {
                        web_name: params.remove("web_name").unwrap(),
                        page_name: params.remove("page_name").unwrap()
                    }

                } else {
                    Route::Invalid
                }
            },
            _ => Route::Invalid
        }
    }
}
