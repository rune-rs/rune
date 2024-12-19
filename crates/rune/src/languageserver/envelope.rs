//! Types to deserialize.

use core::fmt;

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::String;
use serde::{Deserialize, Serialize};

#[derive(Debug, TryClone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub(super) enum RequestId {
    Number(u64),
    String(String),
}

#[derive(Debug, TryClone, Deserialize)]
pub(super) struct IncomingMessage {
    #[allow(unused)]
    pub(super) jsonrpc: V2,
    pub(super) id: Option<RequestId>,
    pub(super) method: String,
    #[serde(default)]
    #[try_clone(with = Clone::clone)]
    pub(super) params: serde_json::Value,
}

#[derive(Debug, TryClone, Serialize)]
#[try_clone(bound = {T: TryClone})]
pub(super) struct NotificationMessage<T> {
    pub(super) jsonrpc: V2,
    pub(super) method: &'static str,
    pub(super) params: T,
}

#[derive(Debug, TryClone, Serialize, Deserialize)]
#[try_clone(bound = {T: TryClone, D: TryClone})]
pub(super) struct ResponseMessage<'a, T, D> {
    pub(super) jsonrpc: V2,
    // NB: serializing for this is not skipped, since the spec requires it to be
    // `null` in case its absent, in contrast to other fields below which should
    // be entirely optional.
    pub(super) id: Option<RequestId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) result: Option<T>,
    #[serde(borrow, skip_serializing_if = "Option::is_none")]
    pub(super) error: Option<ResponseError<'a, D>>,
}

/// Build a type for known error codes and ensure it's serialized correctly.
macro_rules! code {
    (
        $vis:vis enum $name:ident {
            $($variant:ident = $value:expr),* $(,)?
        }
    ) => {
        #[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq, Hash)]
        #[try_clone(copy)]
        $vis enum $name {
            $($variant,)*
            Unknown(i32),
        }

        impl<'de> Deserialize<'de> for $name {
            #[inline]
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>
            {
                match i32::deserialize(deserializer)? {
                    $($value => Ok($name::$variant),)*
                    other => Ok($name::Unknown(other)),
                }
            }
        }

        impl Serialize for $name {
            #[inline]
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer
            {
                match self {
                    $(Code::$variant => serializer.serialize_i32($value),)*
                    Code::Unknown(value) => serializer.serialize_i32(*value),
                }
            }
        }
    }
}

code! {
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
}

#[derive(Debug, TryClone, Serialize, Deserialize)]
#[try_clone(bound = {D: TryClone})]
pub(super) struct ResponseError<'a, D> {
    pub(super) code: Code,
    pub(super) message: &'a str,
    pub(super) data: Option<D>,
}

#[derive(Debug, PartialEq, TryClone, Clone, Copy, Hash, Eq)]
#[try_clone(copy)]
pub(super) struct V2;

impl serde::Serialize for V2 {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str("2.0")
    }
}

impl<'a> serde::Deserialize<'a> for V2 {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<V2, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = V2;

            #[inline]
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string")
            }

            #[inline]
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

        deserializer.deserialize_identifier(Visitor)
    }
}
