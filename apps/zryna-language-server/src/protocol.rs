use std::{collections::BTreeSet, fmt};

use serde::{
    Deserialize,
    de::{MapAccess, SeqAccess, Visitor},
};
use serde_json::{Value, json};
use zryna_driver::diagnostic_sessions::{DiagnosticQueryResponse, QueryStatus};

const MAX_PROTOCOL_ID_BYTES: usize = 120;
const MAX_SAFE_INTEGER: u64 = (1_u64 << 53) - 1;
const MAX_PROTOCOL_DEPTH: u32 = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RequestId {
    wire: Value,
    internal: String,
}

impl RequestId {
    pub(crate) fn wire(&self) -> &Value {
        &self.wire
    }
    pub(crate) fn internal(&self) -> &str {
        &self.internal
    }
}

#[derive(Debug)]
pub(crate) struct Incoming {
    pub(crate) id: Option<RequestId>,
    pub(crate) method: String,
    pub(crate) params: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireIncoming {
    jsonrpc: String,
    #[serde(default)]
    id: OptionalId,
    method: String,
    #[serde(default)]
    params: Option<ClosedValue>,
}

#[derive(Debug, Default)]
enum OptionalId {
    #[default]
    Missing,
    Present(Value),
}

impl<'de> Deserialize<'de> for OptionalId {
    fn deserialize<Deserializer: serde::Deserializer<'de>>(
        deserializer: Deserializer,
    ) -> Result<Self, Deserializer::Error> {
        Value::deserialize(deserializer).map(Self::Present)
    }
}

pub(crate) fn decode(bytes: &[u8]) -> Result<Incoming, Value> {
    if exceeds_depth(bytes) {
        return Err(invalid_request());
    }
    let wire: WireIncoming = serde_json::from_slice(bytes).map_err(|_| parse_error())?;
    if wire.jsonrpc != "2.0"
        || wire.method.is_empty()
        || wire.method.len() > 128
        || !wire.method.is_ascii()
    {
        return Err(invalid_request());
    }
    let id = match wire.id {
        OptionalId::Missing => None,
        OptionalId::Present(value) => Some(request_id(value).map_err(|()| invalid_request())?),
    };
    Ok(Incoming { id, method: wire.method, params: wire.params.map(|value| value.0) })
}

#[derive(Debug)]
struct ClosedValue(Value);

impl<'de> Deserialize<'de> for ClosedValue {
    fn deserialize<Deserializer: serde::Deserializer<'de>>(
        deserializer: Deserializer,
    ) -> Result<Self, Deserializer::Error> {
        deserializer.deserialize_any(ClosedValueVisitor)
    }
}

struct ClosedValueVisitor;

impl<'de> Visitor<'de> for ClosedValueVisitor {
    type Value = ClosedValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JSON without duplicate object keys")
    }

    fn visit_bool<Error: serde::de::Error>(self, value: bool) -> Result<Self::Value, Error> {
        Ok(ClosedValue(Value::Bool(value)))
    }

    fn visit_i64<Error: serde::de::Error>(self, value: i64) -> Result<Self::Value, Error> {
        Ok(ClosedValue(Value::Number(value.into())))
    }

    fn visit_u64<Error: serde::de::Error>(self, value: u64) -> Result<Self::Value, Error> {
        Ok(ClosedValue(Value::Number(value.into())))
    }

    fn visit_f64<Error: serde::de::Error>(self, value: f64) -> Result<Self::Value, Error> {
        serde_json::Number::from_f64(value)
            .map(Value::Number)
            .map(ClosedValue)
            .ok_or_else(|| Error::custom("non-finite JSON number"))
    }

    fn visit_str<Error: serde::de::Error>(self, value: &str) -> Result<Self::Value, Error> {
        self.visit_string(value.to_owned())
    }

    fn visit_string<Error: serde::de::Error>(self, value: String) -> Result<Self::Value, Error> {
        Ok(ClosedValue(Value::String(value)))
    }

    fn visit_none<Error: serde::de::Error>(self) -> Result<Self::Value, Error> {
        Ok(ClosedValue(Value::Null))
    }

    fn visit_unit<Error: serde::de::Error>(self) -> Result<Self::Value, Error> {
        Ok(ClosedValue(Value::Null))
    }

    fn visit_seq<Access: SeqAccess<'de>>(
        self,
        mut access: Access,
    ) -> Result<Self::Value, Access::Error> {
        let mut values = Vec::new();
        while let Some(value) = access.next_element::<ClosedValue>()? {
            values.push(value.0);
        }
        Ok(ClosedValue(Value::Array(values)))
    }

    fn visit_map<Access: MapAccess<'de>>(
        self,
        mut access: Access,
    ) -> Result<Self::Value, Access::Error> {
        let mut keys = BTreeSet::new();
        let mut values = serde_json::Map::new();
        while let Some(key) = access.next_key::<String>()? {
            if !keys.insert(key.clone()) {
                return Err(serde::de::Error::custom("duplicate JSON object key"));
            }
            let value = access.next_value::<ClosedValue>()?;
            values.insert(key, value.0);
        }
        Ok(ClosedValue(Value::Object(values)))
    }
}

fn exceeds_depth(bytes: &[u8]) -> bool {
    let mut depth = 0_u32;
    let mut in_string = false;
    let mut escaped = false;
    for byte in bytes {
        if in_string {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match *byte {
            b'"' => in_string = true,
            b'{' | b'[' => {
                depth += 1;
                if depth > MAX_PROTOCOL_DEPTH {
                    return true;
                }
            }
            b'}' | b']' if depth > 0 => depth -= 1,
            _ => {}
        }
    }
    false
}

fn request_id(value: Value) -> Result<RequestId, ()> {
    let internal = match &value {
        Value::String(value)
            if !value.is_empty() && value.len() <= MAX_PROTOCOL_ID_BYTES && value.is_ascii() =>
        {
            format!("s:{value}")
        }
        Value::Number(value) => {
            if let Some(number) = value.as_u64().filter(|value| *value <= MAX_SAFE_INTEGER) {
                format!("n:{number}")
            } else if let Some(number) =
                value.as_i64().filter(|value| value.unsigned_abs() <= MAX_SAFE_INTEGER)
            {
                format!("n:{number}")
            } else {
                return Err(());
            }
        }
        _ => return Err(()),
    };
    Ok(RequestId { wire: value, internal })
}

pub(crate) fn decode_id(value: Value) -> Option<RequestId> {
    request_id(value).ok()
}

pub(crate) fn response(id: &RequestId, result: &Value) -> Value {
    json!({"jsonrpc":"2.0","id":id.wire(),"result":result})
}

pub(crate) fn error(id: Option<&RequestId>, code: i32, message: &'static str) -> Value {
    json!({"jsonrpc":"2.0","id":id.map_or(Value::Null, |id| id.wire().clone()),"error":{"code":code,"message":message}})
}

pub(crate) fn notification(method: &'static str, params: &Value) -> Value {
    json!({"jsonrpc":"2.0","method":method,"params":params})
}

pub(crate) fn log_invalid() -> Value {
    log_message("rejected malformed language-server notification")
}

pub(crate) fn log_message(message: &str) -> Value {
    notification("window/logMessage", &json!({"type":1,"message":message}))
}

pub(crate) fn response_message(response: &DiagnosticQueryResponse) -> &'static str {
    match response.status() {
        QueryStatus::Cancelled => "diagnostic request cancelled",
        QueryStatus::Stale => "diagnostic request became stale",
        QueryStatus::OverBudget => "diagnostic request exceeded a limit",
        _ => "diagnostic analysis unavailable",
    }
}

pub(crate) fn empty_params(params: Option<&Value>) -> bool {
    params.is_none_or(|value| {
        value.is_null() || value.as_object().is_some_and(serde_json::Map::is_empty)
    })
}

pub(crate) fn parse_error() -> Value {
    error(None, -32700, "Parse error")
}
pub(crate) fn invalid_request() -> Value {
    error(None, -32600, "Invalid Request")
}
pub(crate) fn invalid_params(id: Option<&RequestId>) -> Value {
    error(id, -32602, "Invalid params")
}
pub(crate) fn method_not_found(id: Option<&RequestId>) -> Value {
    error(id, -32601, "Method not found")
}
