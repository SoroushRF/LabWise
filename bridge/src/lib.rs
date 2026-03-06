//! # LabWise Bridge
//!
//! WASM interface layer between the Rust Kernel and the JavaScript Projection.
//! All functions exported here are callable from the browser via `wasm-bindgen`.
//!
//! This crate compiles to WebAssembly and serves as the sole communication
//! channel between the deterministic physics engine and the frontend.

use wasm_bindgen::prelude::*;

// ── WASM Integrity Test ─────────────────────────────────────────────
// Step 1.1.4: Simple function to verify the WASM bridge works.

/// Simple addition function for WASM integrity verification.
/// Called from the browser to verify the Rust→WASM→JS pipeline works.
#[wasm_bindgen]
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// Returns a greeting string to test string passing across the WASM boundary.
#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello from LabWise Kernel, {}!", name)
}

// ── Governance Bridge ───────────────────────────────────────────────

/// Validate a circuit netlist JSON string through the Governance Manager.
/// Returns a JSON string with either the validated result or error details.
#[wasm_bindgen]
pub fn validate_circuit(netlist_json: &str) -> String {
    let gov = labwise_kernel::GovernanceManager::new();
    match gov.validate_json(netlist_json) {
        Ok(validated) => {
            // Return a success response with component count
            let response = serde_json::json!({
                "status": "valid",
                "components": validated.netlist.components.len(),
                "nodes": validated.node_ids.len(),
                "message": "Circuit passed all governance checks"
            });
            serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string())
        }
        Err(errors) => {
            let error_messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
            let response = serde_json::json!({
                "status": "rejected",
                "error_count": error_messages.len(),
                "errors": error_messages,
                "message": "400 Bad Physics"
            });
            serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 3), 5);
        assert_eq!(add(-1, 1), 0);
        assert_eq!(add(0, 0), 0);
    }

    #[test]
    fn test_greet() {
        assert_eq!(greet("World"), "Hello from LabWise Kernel, World!");
    }
}
