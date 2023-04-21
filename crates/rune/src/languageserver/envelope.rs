//! Types to deserialize.

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub(super) enum RequestId {
    Number(u64),
    String(String),
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct IncomingMessage<'a> {
    #[allow(unused)]
    pub(super) jsonrpc: V2,
    pub(super) id: Option<RequestId>,
    #[serde(borrow)]
    pub(super) method: &'a str,
    #[serde(default)]
    pub(super) params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct NotificationMessage<T> {
    pub(super) jsonrpc: V2,
    pub(super) method: &'static str,
    pub(super) params: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ResponseMessage<'a, T, D> {
    pub(super) jsonrpc: V2,
    pub(super) id: Option<RequestId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) result: Option<T>,
    #[serde(borrow, skip_serializing_if = "Option::is_none")]
    pub(super) error: Option<ResponseError<'a, D>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) enum Code {
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
pub(super) struct ResponseError<'a, D> {
    pub(super) code: Code,
    pub(super) message: &'a str,
    pub(super) data: Option<D>,
}

#[derive(Debug, PartialEq, Clone, Copy, Hash, Eq)]
pub(super) struct V2;

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
