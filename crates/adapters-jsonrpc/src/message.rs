//! The JSON-RPC 2.0 envelope: requests, notifications, responses, ids.

use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("not a JSON-RPC 2.0 message: {0}")]
    NotAnObject(String),
    #[error("missing or wrong jsonrpc version")]
    Version,
    #[error("message has neither a method nor a result/error")]
    Shapeless,
    #[error("message has both a method and a result/error")]
    Mixed,
    #[error("response has both result and error")]
    BothResultAndError,
    #[error("invalid id: {0}")]
    Id(String),
    #[error("invalid error object: {0}")]
    ErrorObject(String),
    #[error("invalid JSON: {0}")]
    Json(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RequestId {
    Number(i64),
    String(String),
}

impl RequestId {
    fn to_value(&self) -> Value {
        match self {
            Self::Number(number) => Value::from(*number),
            Self::String(string) => Value::from(string.as_str()),
        }
    }

    fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::Number(number) => number.as_i64().map(Self::Number),
            Value::String(string) => Some(Self::String(string.clone())),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub id: RequestId,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Notification {
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    pub id: Option<RequestId>,
    pub result: Option<Value>,
    pub error: Option<ErrorObject>,
}

impl Response {
    pub fn result(id: RequestId, result: Value) -> Self {
        Self {
            id: Some(id),
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<RequestId>, error: ErrorObject) -> Self {
        Self {
            id,
            result: None,
            error: Some(error),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ErrorObject {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

impl ErrorObject {
    pub fn to_value(&self) -> Value {
        let mut object = Map::new();
        object.insert("code".into(), Value::from(self.code));
        object.insert("message".into(), Value::from(self.message.as_str()));
        if let Some(data) = &self.data {
            object.insert("data".into(), data.clone());
        }
        Value::Object(object)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Request(Request),
    Notification(Notification),
    Response(Response),
}

impl Message {
    pub fn to_value(&self) -> Value {
        match self {
            Self::Request(request) => request.to_value(),
            Self::Notification(notification) => notification.to_value(),
            Self::Response(response) => response.to_value(),
        }
    }
}

impl Request {
    pub fn to_value(&self) -> Value {
        let mut object = Map::new();
        object.insert("jsonrpc".into(), Value::from("2.0"));
        object.insert("id".into(), self.id.to_value());
        object.insert("method".into(), Value::from(self.method.as_str()));
        if let Some(params) = &self.params {
            object.insert("params".into(), params.clone());
        }
        Value::Object(object)
    }
}

impl Notification {
    pub fn to_value(&self) -> Value {
        let mut object = Map::new();
        object.insert("jsonrpc".into(), Value::from("2.0"));
        object.insert("method".into(), Value::from(self.method.as_str()));
        if let Some(params) = &self.params {
            object.insert("params".into(), params.clone());
        }
        Value::Object(object)
    }
}

impl Response {
    pub fn to_value(&self) -> Value {
        let mut object = Map::new();
        object.insert("jsonrpc".into(), Value::from("2.0"));
        object.insert(
            "id".into(),
            match &self.id {
                Some(id) => id.to_value(),
                None => Value::Null,
            },
        );
        match (&self.result, &self.error) {
            (Some(result), None) => {
                object.insert("result".into(), result.clone());
            }
            (None, Some(error)) => {
                object.insert("error".into(), error.to_value());
            }
            _ => {}
        }
        Value::Object(object)
    }
}

fn params_of(object: &Map<String, Value>) -> Option<Value> {
    object
        .get("params")
        .filter(|params| !params.is_null())
        .cloned()
}

fn parse_error_object(value: &Value) -> Result<ErrorObject, ParseError> {
    let object = value
        .as_object()
        .ok_or_else(|| ParseError::ErrorObject(value.to_string()))?;
    let code = object
        .get("code")
        .and_then(Value::as_i64)
        .ok_or_else(|| ParseError::ErrorObject("missing code".into()))?;
    let message = object
        .get("message")
        .and_then(Value::as_str)
        .ok_or_else(|| ParseError::ErrorObject("missing message".into()))?;
    Ok(ErrorObject {
        code,
        message: message.to_string(),
        data: object.get("data").filter(|data| !data.is_null()).cloned(),
    })
}

pub fn parse_line(line: &str) -> Result<Message, ParseError> {
    let value: Value =
        serde_json::from_str(line).map_err(|error| ParseError::Json(error.to_string()))?;
    parse_value(&value)
}

pub fn parse_value(value: &Value) -> Result<Message, ParseError> {
    let object = value
        .as_object()
        .ok_or_else(|| ParseError::NotAnObject(value.to_string()))?;
    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Err(ParseError::Version);
    }
    let method = object.get("method").and_then(Value::as_str);
    let id = object.get("id");
    let carries_result = object.get("result").is_some() || object.get("error").is_some();
    match (method, id, carries_result) {
        (Some(method), _, false) => {
            let params = params_of(object);
            match id {
                None | Some(Value::Null) => Ok(Message::Notification(Notification {
                    method: method.to_string(),
                    params,
                })),
                Some(id) => {
                    let id =
                        RequestId::from_value(id).ok_or_else(|| ParseError::Id(id.to_string()))?;
                    Ok(Message::Request(Request {
                        id,
                        method: method.to_string(),
                        params,
                    }))
                }
            }
        }
        (None, _, true) => {
            if object.get("result").is_some() && object.get("error").is_some() {
                return Err(ParseError::BothResultAndError);
            }
            let id = match id {
                None | Some(Value::Null) => None,
                Some(id) => {
                    Some(RequestId::from_value(id).ok_or_else(|| ParseError::Id(id.to_string()))?)
                }
            };
            let error = match object.get("error") {
                Some(error) => Some(parse_error_object(error)?),
                None => None,
            };
            Ok(Message::Response(Response {
                id,
                result: object.get("result").cloned(),
                error,
            }))
        }
        (Some(_), _, true) => Err(ParseError::Mixed),
        (None, _, false) => Err(ParseError::Shapeless),
    }
}

#[cfg(test)]
mod tests;
