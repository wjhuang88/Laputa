use std::collections::{HashMap, HashSet};

pub enum DataType {
    String(String),
    Integer(i32),
    Double(f64),
    Array(Vec<DataType>),
    Map(HashMap<DataType, DataType>),
    Set(HashSet<DataType>),
    Boolean(bool),
    Undefined,
    Null
}

impl ToString for DataType {
    fn to_string(&self) -> String {
        match self {
            Self::String(value) => value.to_string(),
            Self::Integer(value) => value.to_string(),
            Self::Double(value) => value.to_string(),
            Self::Array(value) => format!("[{}]", value.iter().map(|v| format!("{},", v.to_string())).collect::<String>()),
            Self::Map(value) => format!("{{{}}}", value.iter().map(|(k, v)| format!("{}: {},", k.to_string(), v.to_string())).collect::<String>()),
            Self::Set(value) => format!("{}", value.iter().map(|v| v.to_string()).collect::<String>()),
            Self::Boolean(value) => value.to_string(),
            Self::Undefined => "undefined".to_string(),
            Self::Null => "null".to_string()
        }
    }
}