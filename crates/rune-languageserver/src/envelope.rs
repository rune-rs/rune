//! Types to deserialize.

use bstr::BStr;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    Number(u64),
    String(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct IncomingMessage<'a> {
    pub jsonrpc: V2,
    pub id: Option<RequestId>,
    #[serde(borrow)]
    pub method: &'a BStr,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct NotificationMessage<T> {
    pub jsonrpc: V2,
    pub method: &'static str,
    pub params: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMessage<T, D> {
    pub jsonrpc: V2,
    pub id: Option<RequestId>,
    pub result: Option<T>,
    pub error: Option<ResponseError<D>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Code {
    ParseError = -32700,
    InvalidRequest = -32600,
    MethodNotFound = -32601,
    InvalidParams = -32602,
    InternalError = -32603,
    ServerErrorStart = -32099,
    ServerErrorEnd = -32000,
    ServerNotInitialized = -32002,
    UnknownErrorCode = -32001,
    RequestCancelled = -32800,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseError<D> {
    pub code: Code,
    pub message: String,
    pub data: Option<D>,
}

#[derive(Debug, PartialEq, Clone, Copy, Hash, Eq)]
pub struct V2;

impl serde::Serialize for V2 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str("2.0")
    }
}

impl<'a> serde::Deserialize<'a> for V2 {
    fn deserialize<D>(deserializer: D) -> Result<V2, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        return deserializer.deserialize_identifier(Visitor);

        struct Visitor;

        impl<'a> serde::de::Visitor<'a> for Visitor {
            type Value = V2;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "2.0" => Ok(V2),
                    _ => Err(serde::de::Error::custom("invalid version")),
                }
            }
        }
    }
}
