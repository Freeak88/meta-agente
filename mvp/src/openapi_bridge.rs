#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct OpenApiSpec {
    pub openapi: String,
    pub info: OpenApiInfo,
    pub paths: HashMap<String, OpenApiPathItem>,
    pub components: Option<OpenApiComponents>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct OpenApiInfo {
    pub title: String,
    pub version: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct OpenApiPathItem {
    pub get: Option<OpenApiOperation>,
    pub post: Option<OpenApiOperation>,
    pub put: Option<OpenApiOperation>,
    pub delete: Option<OpenApiOperation>,
    pub patch: Option<OpenApiOperation>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiOperation {
    pub operation_id: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub parameters: Option<Vec<OpenApiParameter>>,
    pub request_body: Option<OpenApiRequestBody>,
    pub responses: HashMap<String, OpenApiResponse>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct OpenApiParameter {
    pub name: String,
    pub r#in: String,
    pub required: Option<bool>,
    pub schema: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct OpenApiRequestBody {
    pub content: HashMap<String, OpenApiMediaType>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct OpenApiMediaType {
    pub schema: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct OpenApiResponse {
    pub description: String,
    pub content: Option<HashMap<String, OpenApiMediaType>>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct OpenApiComponents {
    pub schemas: Option<HashMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GeneratedCapability {
    pub id: String,
    pub method: String,
    pub path: String,
    pub description: String,
    pub required_params: Vec<String>,
    pub optional_params: Vec<String>,
    pub has_request_body: bool,
}

pub struct OpenApiBridge;

impl OpenApiBridge {
    pub async fn from_url(url: &str) -> Result<OpenApiSpec, String> {
        let response = reqwest::Client::new()
            .get(url)
            .send()
            .await
            .map_err(|err| format!("openapi_fetch_error: {}", err))?;

        if !response.status().is_success() {
            return Err(format!(
                "openapi_http_error: {}",
                response.status().as_u16()
            ));
        }

        response
            .json::<OpenApiSpec>()
            .await
            .map_err(|err| format!("openapi_parse_error: {}", err))
    }

    pub fn generate_capabilities(spec: &OpenApiSpec) -> Vec<GeneratedCapability> {
        let mut capabilities = Vec::new();

        for (path, item) in &spec.paths {
            Self::push_operation(&mut capabilities, path, "GET", &item.get);
            Self::push_operation(&mut capabilities, path, "POST", &item.post);
            Self::push_operation(&mut capabilities, path, "PUT", &item.put);
            Self::push_operation(&mut capabilities, path, "PATCH", &item.patch);
            Self::push_operation(&mut capabilities, path, "DELETE", &item.delete);
        }

        capabilities.sort_by(|left, right| left.id.cmp(&right.id));
        capabilities
    }

    fn push_operation(
        capabilities: &mut Vec<GeneratedCapability>,
        path: &str,
        method: &str,
        operation: &Option<OpenApiOperation>,
    ) {
        if let Some(operation) = operation {
            capabilities.push(Self::operation_to_capability(path, method, operation));
        }
    }

    fn operation_to_capability(
        path: &str,
        method: &str,
        operation: &OpenApiOperation,
    ) -> GeneratedCapability {
        let id = operation
            .operation_id
            .clone()
            .unwrap_or_else(|| Self::fallback_operation_id(method, path));
        let description = operation
            .summary
            .clone()
            .or_else(|| operation.description.clone())
            .unwrap_or_else(|| format!("{} {}", method, path));
        let mut required_params = Vec::new();
        let mut optional_params = Vec::new();

        if let Some(parameters) = &operation.parameters {
            for parameter in parameters {
                if parameter.required.unwrap_or(false) {
                    required_params.push(parameter.name.clone());
                } else {
                    optional_params.push(parameter.name.clone());
                }
            }
        }

        required_params.sort();
        optional_params.sort();

        GeneratedCapability {
            id,
            method: method.to_string(),
            path: path.to_string(),
            description,
            required_params,
            optional_params,
            has_request_body: operation.request_body.is_some(),
        }
    }

    fn fallback_operation_id(method: &str, path: &str) -> String {
        let mut normalized = path
            .trim_matches('/')
            .replace(['/', '-'], "_")
            .replace(['{', '}'], "");
        if normalized.is_empty() {
            normalized = "root".to_string();
        }
        format!("{}_{}", method.to_lowercase(), normalized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn petstore_spec() -> &'static str {
        r#"{
            "openapi": "3.0.0",
            "info": { "title": "Swagger Petstore", "version": "1.0.0" },
            "paths": {
                "/pets": {
                    "get": {
                        "operationId": "listPets",
                        "summary": "List all pets",
                        "parameters": [
                            { "name": "limit", "in": "query", "required": false, "schema": { "type": "integer" } }
                        ],
                        "responses": { "200": { "description": "A paged array of pets" } }
                    },
                    "post": {
                        "operationId": "createPets",
                        "summary": "Create a pet",
                        "requestBody": { "content": { "application/json": { "schema": { "type": "object" } } } },
                        "responses": { "201": { "description": "Null response" } }
                    }
                },
                "/pets/{petId}": {
                    "get": {
                        "operationId": "showPetById",
                        "summary": "Info for a specific pet",
                        "parameters": [
                            { "name": "petId", "in": "path", "required": true, "schema": { "type": "string" } }
                        ],
                        "responses": { "200": { "description": "Expected response to a valid request" } }
                    }
                }
            }
        }"#
    }

    #[test]
    fn parses_openapi_spec() {
        let spec: OpenApiSpec = serde_json::from_str(petstore_spec()).unwrap();

        assert_eq!(spec.openapi, "3.0.0");
        assert_eq!(spec.info.title, "Swagger Petstore");
        assert_eq!(spec.paths.len(), 2);
    }

    #[test]
    fn generates_capabilities_from_openapi_paths() {
        let spec: OpenApiSpec = serde_json::from_str(petstore_spec()).unwrap();
        let capabilities = OpenApiBridge::generate_capabilities(&spec);

        assert_eq!(capabilities.len(), 3);

        let list_pets = capabilities
            .iter()
            .find(|capability| capability.id == "listPets")
            .unwrap();
        assert_eq!(list_pets.method, "GET");
        assert_eq!(list_pets.path, "/pets");
        assert_eq!(list_pets.description, "List all pets");
        assert!(list_pets.optional_params.contains(&"limit".to_string()));
        assert!(!list_pets.has_request_body);

        let create_pets = capabilities
            .iter()
            .find(|capability| capability.id == "createPets")
            .unwrap();
        assert_eq!(create_pets.method, "POST");
        assert!(create_pets.has_request_body);

        let show_pet = capabilities
            .iter()
            .find(|capability| capability.id == "showPetById")
            .unwrap();
        assert_eq!(show_pet.method, "GET");
        assert!(show_pet.required_params.contains(&"petId".to_string()));
    }

    #[test]
    fn generates_fallback_ids_when_operation_id_is_missing() {
        let spec_json = r#"{
            "openapi": "3.0.0",
            "info": { "title": "Fallback", "version": "1.0.0" },
            "paths": {
                "/users/{userId}/profile": {
                    "patch": {
                        "responses": { "200": { "description": "ok" } }
                    }
                }
            }
        }"#;
        let spec: OpenApiSpec = serde_json::from_str(spec_json).unwrap();

        let capabilities = OpenApiBridge::generate_capabilities(&spec);

        assert_eq!(capabilities[0].id, "patch_users_userId_profile");
        assert_eq!(capabilities[0].description, "PATCH /users/{userId}/profile");
    }
}
