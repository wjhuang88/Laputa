use actix::{Actor, Handler, Message as ActixMsg};
use std::io;
use std::path::PathBuf;

#[derive(Clone)]
pub struct RequestInfo {
    pub path: String
}

pub struct ScriptMessage {
    pub request: RequestInfo,
    pub source: Vec<u8>,
    pub file: String
}
impl ActixMsg for ScriptMessage {
    type Result = Result<String, io::Error>;
}

pub trait ScriptHandler : Actor + Handler<ScriptMessage> {
    type SourceType;
    type ValueType;
    fn exec(&mut self, source: Self::SourceType, file_name: String) -> io::Result<Self::ValueType>;
}

#[derive(Clone)]
pub struct ScriptFile {
    pub file_path: PathBuf,
    pub script_type: String
}
impl ScriptFile {
    pub fn route_path(&self) -> String {
        self.file_path.file_stem()
            .map(|s| s.to_str().unwrap_or(""))
            .unwrap_or("").to_string()
    }

    pub fn get_source(&self) -> io::Result<Vec<u8>> {
        std::fs::read(self.file_path.clone())
    }

    pub fn get_file_name(&self) -> String {
        self.file_path.to_str().unwrap_or("<unknown>").to_string()
    }
}