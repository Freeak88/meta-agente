#![allow(dead_code)]

use std::collections::HashSet;

use wasmtime::{Engine, Instance, Module, Store};

#[derive(Debug, Clone, PartialEq)]
pub enum WasmRuntimeError {
    InvalidOpl(String),
    EmptyAgent,
    Compile(String),
    Instantiate(String),
    SandboxViolation(String),
    StepNotFound(String),
    Execute(String),
}

pub struct WasmRuntime {
    store: Store<WasmState>,
    instance: Instance,
    step_ids: HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct WasmState {
    pub memory: Vec<u8>,
}

impl WasmRuntime {
    pub fn compile(opl: &str) -> Result<Vec<u8>, WasmRuntimeError> {
        let ast = crate::dsl::parse_opl(opl).map_err(WasmRuntimeError::InvalidOpl)?;
        let step_count = ast.agent.steps.len();

        if step_count == 0 {
            return Err(WasmRuntimeError::EmptyAgent);
        }

        let module = format!(
            r#"
            (module
              (func (export "execute_step") (param $step_index i32) (param $input_len i32) (result i32)
                local.get $step_index
                i32.const 0
                i32.ge_s
              )
              (func (export "step_count") (result i32)
                i32.const {step_count}
              )
            )
            "#
        );

        wat::parse_str(module).map_err(|error| WasmRuntimeError::Compile(error.to_string()))
    }

    pub fn instantiate(wasm: &[u8], opl: &str) -> Result<Self, WasmRuntimeError> {
        if !Self::is_sandboxed(wasm)? {
            return Err(WasmRuntimeError::SandboxViolation(
                "module declares host imports".to_string(),
            ));
        }

        let ast = crate::dsl::parse_opl(opl).map_err(WasmRuntimeError::InvalidOpl)?;
        let step_ids = ast
            .agent
            .steps
            .iter()
            .map(|step| step.id.clone())
            .collect::<HashSet<_>>();
        let engine = Engine::default();
        let module = Module::from_binary(&engine, wasm)
            .map_err(|error| WasmRuntimeError::Instantiate(error.to_string()))?;
        let mut store = Store::new(&engine, WasmState { memory: Vec::new() });
        let instance = Instance::new(&mut store, &module, &[])
            .map_err(|error| WasmRuntimeError::Instantiate(error.to_string()))?;

        Ok(Self {
            store,
            instance,
            step_ids,
        })
    }

    pub fn execute(&mut self, step_id: &str, input: &str) -> Result<String, WasmRuntimeError> {
        if !self.step_ids.contains(step_id) {
            return Err(WasmRuntimeError::StepNotFound(step_id.to_string()));
        }

        let step_index = self
            .step_ids
            .iter()
            .position(|candidate| candidate == step_id)
            .unwrap_or(0) as i32;
        let execute = self
            .instance
            .get_typed_func::<(i32, i32), i32>(&mut self.store, "execute_step")
            .map_err(|error| WasmRuntimeError::Execute(error.to_string()))?;
        let code = execute
            .call(&mut self.store, (step_index, input.len() as i32))
            .map_err(|error| WasmRuntimeError::Execute(error.to_string()))?;

        if code == 1 {
            Ok(serde_json::json!({
                "status": "ok",
                "step_id": step_id,
                "input_len": input.len()
            })
            .to_string())
        } else {
            Err(WasmRuntimeError::Execute(format!(
                "wasm returned non-success code: {}",
                code
            )))
        }
    }

    pub fn step_count(wasm: &[u8]) -> Result<i32, WasmRuntimeError> {
        let engine = Engine::default();
        let module = Module::from_binary(&engine, wasm)
            .map_err(|error| WasmRuntimeError::Instantiate(error.to_string()))?;
        let mut store = Store::new(&engine, WasmState { memory: Vec::new() });
        let instance = Instance::new(&mut store, &module, &[])
            .map_err(|error| WasmRuntimeError::Instantiate(error.to_string()))?;
        let step_count = instance
            .get_typed_func::<(), i32>(&mut store, "step_count")
            .map_err(|error| WasmRuntimeError::Execute(error.to_string()))?;

        step_count
            .call(&mut store, ())
            .map_err(|error| WasmRuntimeError::Execute(error.to_string()))
    }

    pub fn is_sandboxed(wasm: &[u8]) -> Result<bool, WasmRuntimeError> {
        let engine = Engine::default();
        let module = Module::from_binary(&engine, wasm)
            .map_err(|error| WasmRuntimeError::Instantiate(error.to_string()))?;
        let sandboxed = module.imports().next().is_none();

        Ok(sandboxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_opl() -> &'static str {
        r#"
CREATE AGENT edge_test
FOR "Validar WASM edge"
WITH
  domain = "testing",
  autonomy = "full"

STEP ping
  USES http_ping
  RISK LOW
"#
    }

    #[test]
    fn wasm_compile_contract() {
        let wasm = WasmRuntime::compile(minimal_opl()).expect("OPL debe compilar a WASM");

        assert!(!wasm.is_empty());
        assert_eq!(WasmRuntime::step_count(&wasm).unwrap(), 1);
    }

    #[test]
    fn wasm_execute_contract() {
        let wasm = WasmRuntime::compile(minimal_opl()).unwrap();
        let mut runtime = WasmRuntime::instantiate(&wasm, minimal_opl()).unwrap();
        let output = runtime.execute("ping", r#"{"ok":true}"#).unwrap();
        let json: serde_json::Value = serde_json::from_str(&output).unwrap();

        assert_eq!(json["status"], "ok");
        assert_eq!(json["step_id"], "ping");
        assert_eq!(json["input_len"], 11);
    }

    #[test]
    fn wasm_sandbox_contract_rejects_host_imports() {
        let unsafe_wasm = wat::parse_str(
            r#"
            (module
              (import "wasi_snapshot_preview1" "fd_write" (func $fd_write))
              (func (export "execute_step") (result i32)
                i32.const 1
              )
            )
            "#,
        )
        .unwrap();

        assert_eq!(WasmRuntime::is_sandboxed(&unsafe_wasm), Ok(false));
        assert!(matches!(
            WasmRuntime::instantiate(&unsafe_wasm, minimal_opl()),
            Err(WasmRuntimeError::SandboxViolation(_))
        ));
    }
}
