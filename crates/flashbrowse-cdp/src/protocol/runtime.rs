//! `Runtime` domain types (evaluate, callFunctionOn).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::CdpError;
use crate::transport::CdpClient;

/// Params for `Runtime.evaluate`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateParams {
    pub expression: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub await_promise: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_by_value: Option<bool>,
}

/// Params for `Runtime.callFunctionOn`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CallFunctionOnParams {
    pub function_declaration: String,
    pub object_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_by_value: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub await_promise: Option<bool>,
}

/// Runtime value wrapper.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeValue {
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub value: Option<Value>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Response of `Runtime.evaluate`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateResponse {
    pub result: RuntimeValue,
    #[serde(default)]
    pub exception_details: Option<Value>,
}

/// Response of `Runtime.callFunctionOn`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallFunctionOnResponse {
    pub result: RuntimeValue,
    #[serde(default)]
    pub exception_details: Option<Value>,
}

/// `Runtime.evaluate` with `returnByValue: true` (markdown extraction, URL/title).
pub async fn evaluate(
    client: &CdpClient,
    session_id: &str,
    expression: &str,
) -> Result<EvaluateResponse, CdpError> {
    client
        .send_typed(
            Some(session_id),
            "Runtime.evaluate",
            &EvaluateParams {
                expression: expression.to_string(),
                await_promise: Some(false),
                return_by_value: Some(true),
            },
        )
        .await
}

/// `Runtime.callFunctionOn` - run a function against a resolved JS object.
pub async fn call_function_on(
    client: &CdpClient,
    session_id: &str,
    object_id: &str,
    function_declaration: &str,
    args: Vec<Value>,
) -> Result<CallFunctionOnResponse, CdpError> {
    client
        .send_typed(
            Some(session_id),
            "Runtime.callFunctionOn",
            &CallFunctionOnParams {
                function_declaration: function_declaration.to_string(),
                object_id: object_id.to_string(),
                arguments: Some(args),
                return_by_value: Some(true),
                await_promise: Some(false),
            },
        )
        .await
}
