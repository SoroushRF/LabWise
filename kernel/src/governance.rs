//! # Governance Manager
//!
//! The Governance Manager is the first line of defense. It pre-validates
//! all incoming circuit netlists against physical laws before they reach
//! the MNA solver. If a circuit violates any physical constraint, it is
//! **rejected** — no exceptions.
//!
//! ## Checks performed:
//! 1. JSON schema validation (via serde deserialization)
//! 2. Component parameter validation (physics-based, universal)
//! 3. Short-circuit detection (VCC to GND through zero resistance)
//! 4. Pin count verification
//! 5. Floating node detection (nodes with only one connection)
//! 6. Ground reference check
//! 7. Power source check

use crate::component_library::{spec_from_type, ComponentSpec, ComponentValidationError};
use crate::netlist::{ComponentType, Netlist};
use std::collections::{HashMap, HashSet};

/// The governance manager validates netlists against physical laws.
pub struct GovernanceManager;

/// A netlist that has passed all governance checks.
/// This is the only way to create a validated netlist — you must go
/// through the GovernanceManager.
#[derive(Debug, Clone)]
pub struct ValidatedNetlist {
    /// The original netlist data
    pub netlist: Netlist,
    /// Generated component specs (from universal validation)
    pub component_specs: HashMap<String, ComponentSpec>,
    /// All unique node IDs in the circuit
    pub node_ids: HashSet<String>,
}

/// Errors returned when a netlist fails governance validation.
#[derive(Debug, Clone)]
pub enum PhysicsError {
    /// Direct short circuit detected between two nodes
    ShortCircuit {
        node_a: String,
        node_b: String,
    },
    /// Same pin used in conflicting connections
    PinConflict {
        component: String,
        pin: String,
    },
    /// Applied voltage exceeds component rating
    OverVoltage {
        component: String,
        applied: f64,
        max: f64,
    },
    /// Applied current exceeds component rating
    OverCurrent {
        component: String,
        applied: f64,
        max: f64,
    },
    /// Component parameter validation failed
    InvalidComponent(ComponentValidationError),
    /// Generic netlist structure error
    InvalidNetlist(String),
    /// A node has only one connection (floating)
    FloatingNode {
        node: String,
    },
    /// Circuit has no ground reference
    NoGroundReference,
    /// Circuit has no power source
    NoPowerSource,
}

impl std::fmt::Display for PhysicsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PhysicsError::ShortCircuit { node_a, node_b } => {
                write!(f, "SHORT CIRCUIT: Direct short between {} and {}", node_a, node_b)
            }
            PhysicsError::PinConflict { component, pin } => {
                write!(f, "PIN CONFLICT: Pin {} on component {} has conflicting connections", pin, component)
            }
            PhysicsError::OverVoltage { component, applied, max } => {
                write!(f, "OVER-VOLTAGE: Component {} sees {}V but is rated for {}V max", component, applied, max)
            }
            PhysicsError::OverCurrent { component, applied, max } => {
                write!(f, "OVER-CURRENT: Component {} draws {}A but is rated for {}A max", component, applied, max)
            }
            PhysicsError::InvalidComponent(err) => {
                write!(f, "{}", err)
            }
            PhysicsError::InvalidNetlist(msg) => {
                write!(f, "INVALID NETLIST: {}", msg)
            }
            PhysicsError::FloatingNode { node } => {
                write!(f, "FLOATING NODE: Node '{}' has only one connection", node)
            }
            PhysicsError::NoGroundReference => {
                write!(f, "NO GROUND: Circuit has no ground reference node")
            }
            PhysicsError::NoPowerSource => {
                write!(f, "NO POWER: Circuit has no power source")
            }
        }
    }
}

impl GovernanceManager {
    /// Create a new GovernanceManager.
    pub fn new() -> Self {
        GovernanceManager
    }

    /// Validate a raw JSON string as a circuit netlist.
    ///
    /// Returns a `ValidatedNetlist` if all checks pass, or a list of
    /// all physics errors found.
    pub fn validate_json(&self, netlist_json: &str) -> Result<ValidatedNetlist, Vec<PhysicsError>> {
        // Step 1: Parse JSON
        let netlist = match Netlist::from_json(netlist_json) {
            Ok(n) => n,
            Err(e) => {
                return Err(vec![PhysicsError::InvalidNetlist(format!(
                    "JSON parse error: {}",
                    e
                ))]);
            }
        };

        self.validate(netlist)
    }

    /// Validate a parsed Netlist.
    pub fn validate(&self, netlist: Netlist) -> Result<ValidatedNetlist, Vec<PhysicsError>> {
        let mut errors = Vec::new();

        // Step 1: Basic structural validation
        if netlist.components.is_empty() {
            errors.push(PhysicsError::InvalidNetlist(
                "Circuit has no components".to_string(),
            ));
        }

        // Step 2: Validate each component's parameters using universal physics rules
        let mut component_specs = HashMap::new();
        for component in &netlist.components {
            match spec_from_type(component.component_type, component.value, &component.id) {
                Ok(spec) => {
                    component_specs.insert(component.id.clone(), spec);
                }
                Err(validation_err) => {
                    errors.push(PhysicsError::InvalidComponent(validation_err));
                }
            }
        }

        // Step 3: Build node connection map
        let mut node_connections: HashMap<String, Vec<String>> = HashMap::new();
        for component in &netlist.components {
            for pin in &component.pins {
                node_connections
                    .entry(pin.node.clone())
                    .or_default()
                    .push(format!("{}:{}", component.id, pin.pin_id));
            }
        }
        // Also include explicit connections
        for conn in &netlist.connections {
            node_connections
                .entry(conn.from_node.clone())
                .or_default()
                .push(format!("wire:{}", conn.to_node));
            node_connections
                .entry(conn.to_node.clone())
                .or_default()
                .push(format!("wire:{}", conn.from_node));
        }

        let node_ids: HashSet<String> = node_connections.keys().cloned().collect();

        // Step 4: Check for ground reference
        let has_ground = node_ids.iter().any(|n| {
            let lower = n.to_lowercase();
            lower == "gnd" || lower == "ground" || lower == "0"
        });
        if !has_ground {
            errors.push(PhysicsError::NoGroundReference);
        }

        // Step 5: Check for power source
        let has_power = netlist
            .components
            .iter()
            .any(|c| c.component_type == ComponentType::Battery);
        if !has_power {
            errors.push(PhysicsError::NoPowerSource);
        }

        // Step 6: Short circuit detection
        self.check_short_circuits(&netlist, &mut errors);

        // Step 7: Floating node detection
        for (node, connections) in &node_connections {
            let lower = node.to_lowercase();
            // Skip GND and VCC — they're allowed to have any number of connections
            if lower == "gnd" || lower == "ground" || lower == "vcc" || lower == "0" {
                continue;
            }
            if connections.len() < 2 {
                errors.push(PhysicsError::FloatingNode {
                    node: node.clone(),
                });
            }
        }

        // Step 8: Pin count validation
        for component in &netlist.components {
            if let Some(spec) = component_specs.get(&component.id) {
                if component.pins.len() != spec.pin_count {
                    errors.push(PhysicsError::InvalidNetlist(format!(
                        "Component {} ({}) has {} pins but expected {}",
                        component.id,
                        spec.name,
                        component.pins.len(),
                        spec.pin_count
                    )));
                }
            }
        }

        if errors.is_empty() {
            Ok(ValidatedNetlist {
                netlist,
                component_specs,
                node_ids,
            })
        } else {
            Err(errors)
        }
    }

    /// Check for short circuits — VCC/power directly connected to GND
    /// through zero-resistance paths (wires only).
    fn check_short_circuits(&self, netlist: &Netlist, errors: &mut Vec<PhysicsError>) {
        // Build adjacency list of nodes connected through wires only
        let mut wire_graph: HashMap<String, HashSet<String>> = HashMap::new();

        for component in &netlist.components {
            if component.component_type == ComponentType::Wire {
                if component.pins.len() == 2 {
                    let n1 = &component.pins[0].node;
                    let n2 = &component.pins[1].node;
                    wire_graph.entry(n1.clone()).or_default().insert(n2.clone());
                    wire_graph.entry(n2.clone()).or_default().insert(n1.clone());
                }
            }
        }

        for conn in &netlist.connections {
            wire_graph
                .entry(conn.from_node.clone())
                .or_default()
                .insert(conn.to_node.clone());
            wire_graph
                .entry(conn.to_node.clone())
                .or_default()
                .insert(conn.from_node.clone());
        }

        // BFS from any VCC/power node to see if GND is reachable through wires only
        let power_nodes: Vec<String> = wire_graph
            .keys()
            .filter(|n| {
                let lower = n.to_lowercase();
                lower == "vcc" || lower == "5v" || lower == "3v3" || lower == "9v"
            })
            .cloned()
            .collect();

        let ground_nodes: HashSet<String> = wire_graph
            .keys()
            .filter(|n| {
                let lower = n.to_lowercase();
                lower == "gnd" || lower == "ground" || lower == "0"
            })
            .cloned()
            .collect();

        for power in &power_nodes {
            // BFS
            let mut visited = HashSet::new();
            let mut queue = vec![power.clone()];
            visited.insert(power.clone());

            while let Some(current) = queue.pop() {
                if ground_nodes.contains(&current) {
                    errors.push(PhysicsError::ShortCircuit {
                        node_a: power.clone(),
                        node_b: current,
                    });
                    break;
                }
                if let Some(neighbors) = wire_graph.get(&current) {
                    for neighbor in neighbors {
                        if visited.insert(neighbor.clone()) {
                            queue.push(neighbor.clone());
                        }
                    }
                }
            }
        }
    }
}

impl Default for GovernanceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::netlist::*;

    fn make_simple_valid_circuit() -> Netlist {
        Netlist {
            components: vec![
                Component {
                    id: "BAT1".to_string(),
                    component_type: ComponentType::Battery,
                    pins: vec![
                        Pin { pin_id: "positive".to_string(), node: "VCC".to_string() },
                        Pin { pin_id: "negative".to_string(), node: "GND".to_string() },
                    ],
                    tolerance: 0.05,
                    electrical_limits: ElectricalLimits {
                        voltage_max: Some(9.6),
                        current_max: Some(0.5),
                    },
                    value: Some(9.0),
                },
                Component {
                    id: "R1".to_string(),
                    component_type: ComponentType::Resistor,
                    pins: vec![
                        Pin { pin_id: "pin1".to_string(), node: "VCC".to_string() },
                        Pin { pin_id: "pin2".to_string(), node: "n1".to_string() },
                    ],
                    tolerance: 0.05,
                    electrical_limits: ElectricalLimits {
                        voltage_max: Some(200.0),
                        current_max: Some(0.25),
                    },
                    value: Some(330.0),
                },
                Component {
                    id: "LED1".to_string(),
                    component_type: ComponentType::Led,
                    pins: vec![
                        Pin { pin_id: "anode".to_string(), node: "n1".to_string() },
                        Pin { pin_id: "cathode".to_string(), node: "GND".to_string() },
                    ],
                    tolerance: 0.1,
                    electrical_limits: ElectricalLimits {
                        voltage_max: Some(2.2),
                        current_max: Some(0.02),
                    },
                    value: None,
                },
            ],
            connections: vec![],
            metadata: Metadata {
                name: Some("Simple LED Circuit".to_string()),
                description: Some("9V battery with 330 ohm resistor and red LED".to_string()),
                source: None,
            },
        }
    }

    #[test]
    fn test_valid_circuit_passes() {
        let gov = GovernanceManager::new();
        let netlist = make_simple_valid_circuit();
        let result = gov.validate(netlist);
        assert!(result.is_ok(), "Valid circuit should pass governance: {:?}", result.err());
    }

    #[test]
    fn test_any_resistor_value_accepted() {
        let gov = GovernanceManager::new();
        // Use a 2.2kΩ resistor — NOT in any specific kit
        let mut netlist = make_simple_valid_circuit();
        netlist.components[1].value = Some(2200.0);
        let result = gov.validate(netlist);
        assert!(result.is_ok(), "2.2kΩ resistor should be accepted: {:?}", result.err());
    }

    #[test]
    fn test_any_battery_voltage_accepted() {
        let gov = GovernanceManager::new();
        // Use a 12V battery — not the standard 9V
        let mut netlist = make_simple_valid_circuit();
        netlist.components[0].value = Some(12.0);
        let result = gov.validate(netlist);
        assert!(result.is_ok(), "12V battery should be accepted: {:?}", result.err());
    }

    #[test]
    fn test_zero_resistance_rejected() {
        let gov = GovernanceManager::new();
        let mut netlist = make_simple_valid_circuit();
        netlist.components[1].value = Some(0.0);
        let result = gov.validate(netlist);
        assert!(result.is_err(), "0Ω resistor should be rejected");
    }

    #[test]
    fn test_negative_resistance_rejected() {
        let gov = GovernanceManager::new();
        let mut netlist = make_simple_valid_circuit();
        netlist.components[1].value = Some(-100.0);
        let result = gov.validate(netlist);
        assert!(result.is_err(), "Negative resistance should be rejected");
    }

    #[test]
    fn test_unsafe_voltage_rejected() {
        let gov = GovernanceManager::new();
        let mut netlist = make_simple_valid_circuit();
        netlist.components[0].value = Some(240.0); // Mains voltage!
        let result = gov.validate(netlist);
        assert!(result.is_err(), "240V should be rejected as unsafe");
    }

    #[test]
    fn test_empty_circuit_rejected() {
        let gov = GovernanceManager::new();
        let netlist = Netlist {
            components: vec![],
            connections: vec![],
            metadata: Metadata { name: None, description: None, source: None },
        };
        let result = gov.validate(netlist);
        assert!(result.is_err());
    }

    #[test]
    fn test_short_circuit_detected() {
        let gov = GovernanceManager::new();
        let netlist = Netlist {
            components: vec![
                Component {
                    id: "BAT1".to_string(),
                    component_type: ComponentType::Battery,
                    pins: vec![
                        Pin { pin_id: "positive".to_string(), node: "VCC".to_string() },
                        Pin { pin_id: "negative".to_string(), node: "GND".to_string() },
                    ],
                    tolerance: 0.05,
                    electrical_limits: ElectricalLimits {
                        voltage_max: Some(9.6),
                        current_max: Some(0.5),
                    },
                    value: Some(9.0),
                },
                // Wire directly from VCC to GND = SHORT CIRCUIT
                Component {
                    id: "W1".to_string(),
                    component_type: ComponentType::Wire,
                    pins: vec![
                        Pin { pin_id: "pin1".to_string(), node: "VCC".to_string() },
                        Pin { pin_id: "pin2".to_string(), node: "GND".to_string() },
                    ],
                    tolerance: 0.0,
                    electrical_limits: ElectricalLimits {
                        voltage_max: Some(300.0),
                        current_max: Some(3.0),
                    },
                    value: Some(0.0),
                },
            ],
            connections: vec![],
            metadata: Metadata { name: None, description: None, source: None },
        };
        let result = gov.validate(netlist);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| matches!(e, PhysicsError::ShortCircuit { .. })),
            "Should detect short circuit: {:?}",
            errors
        );
    }

    #[test]
    fn test_no_ground_rejected() {
        let gov = GovernanceManager::new();
        let netlist = Netlist {
            components: vec![
                Component {
                    id: "BAT1".to_string(),
                    component_type: ComponentType::Battery,
                    pins: vec![
                        Pin { pin_id: "positive".to_string(), node: "VCC".to_string() },
                        Pin { pin_id: "negative".to_string(), node: "n1".to_string() },
                    ],
                    tolerance: 0.05,
                    electrical_limits: ElectricalLimits {
                        voltage_max: Some(9.6),
                        current_max: Some(0.5),
                    },
                    value: Some(9.0),
                },
                Component {
                    id: "R1".to_string(),
                    component_type: ComponentType::Resistor,
                    pins: vec![
                        Pin { pin_id: "pin1".to_string(), node: "VCC".to_string() },
                        Pin { pin_id: "pin2".to_string(), node: "n1".to_string() },
                    ],
                    tolerance: 0.05,
                    electrical_limits: ElectricalLimits {
                        voltage_max: Some(200.0),
                        current_max: Some(0.25),
                    },
                    value: Some(1000.0),
                },
            ],
            connections: vec![],
            metadata: Metadata { name: None, description: None, source: None },
        };
        let result = gov.validate(netlist);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| matches!(e, PhysicsError::NoGroundReference)),
            "Should detect missing ground: {:?}",
            errors
        );
    }

    #[test]
    fn test_invalid_json_rejected() {
        let gov = GovernanceManager::new();
        let result = gov.validate_json("{ this is not valid json }");
        assert!(result.is_err());
    }

    // ── Stress Test #8: No Power Source ────────────────────────────
    #[test]
    fn test_no_power_source_rejected() {
        let gov = GovernanceManager::new();
        let netlist = Netlist {
            components: vec![
                // Resistor between two nodes, but no battery to drive current
                Component {
                    id: "R1".to_string(),
                    component_type: ComponentType::Resistor,
                    pins: vec![
                        Pin { pin_id: "pin1".to_string(), node: "VCC".to_string() },
                        Pin { pin_id: "pin2".to_string(), node: "GND".to_string() },
                    ],
                    tolerance: 0.05,
                    electrical_limits: ElectricalLimits {
                        voltage_max: Some(200.0),
                        current_max: Some(0.25),
                    },
                    value: Some(1000.0),
                },
            ],
            connections: vec![],
            metadata: Metadata { name: None, description: None, source: None },
        };
        let result = gov.validate(netlist);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| matches!(e, PhysicsError::NoPowerSource)),
            "Should detect missing power source: {:?}",
            errors
        );
    }

    // ── Stress Test #9: Floating Node ─────────────────────────────
    #[test]
    fn test_floating_node_rejected() {
        let gov = GovernanceManager::new();
        let netlist = Netlist {
            components: vec![
                Component {
                    id: "BAT1".to_string(),
                    component_type: ComponentType::Battery,
                    pins: vec![
                        Pin { pin_id: "positive".to_string(), node: "VCC".to_string() },
                        Pin { pin_id: "negative".to_string(), node: "GND".to_string() },
                    ],
                    tolerance: 0.05,
                    electrical_limits: ElectricalLimits {
                        voltage_max: Some(9.6),
                        current_max: Some(0.5),
                    },
                    value: Some(9.0),
                },
                // R1 connects VCC to "n1", but nothing else connects to "n1" — it's floating
                Component {
                    id: "R1".to_string(),
                    component_type: ComponentType::Resistor,
                    pins: vec![
                        Pin { pin_id: "pin1".to_string(), node: "VCC".to_string() },
                        Pin { pin_id: "pin2".to_string(), node: "n1".to_string() },
                    ],
                    tolerance: 0.05,
                    electrical_limits: ElectricalLimits {
                        voltage_max: Some(200.0),
                        current_max: Some(0.25),
                    },
                    value: Some(1000.0),
                },
            ],
            connections: vec![],
            metadata: Metadata { name: None, description: None, source: None },
        };
        let result = gov.validate(netlist);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| matches!(e, PhysicsError::FloatingNode { .. })),
            "Should detect floating node n1: {:?}",
            errors
        );
    }

    // ── Stress Test #10: Wrong Pin Count ──────────────────────────
    #[test]
    fn test_wrong_pin_count_rejected() {
        let gov = GovernanceManager::new();
        let netlist = Netlist {
            components: vec![
                Component {
                    id: "BAT1".to_string(),
                    component_type: ComponentType::Battery,
                    pins: vec![
                        Pin { pin_id: "positive".to_string(), node: "VCC".to_string() },
                        Pin { pin_id: "negative".to_string(), node: "GND".to_string() },
                    ],
                    tolerance: 0.05,
                    electrical_limits: ElectricalLimits {
                        voltage_max: Some(9.6),
                        current_max: Some(0.5),
                    },
                    value: Some(9.0),
                },
                // Resistor with 3 pins — physically impossible
                Component {
                    id: "R1".to_string(),
                    component_type: ComponentType::Resistor,
                    pins: vec![
                        Pin { pin_id: "pin1".to_string(), node: "VCC".to_string() },
                        Pin { pin_id: "pin2".to_string(), node: "GND".to_string() },
                        Pin { pin_id: "pin3".to_string(), node: "n1".to_string() },
                    ],
                    tolerance: 0.05,
                    electrical_limits: ElectricalLimits {
                        voltage_max: Some(200.0),
                        current_max: Some(0.25),
                    },
                    value: Some(1000.0),
                },
            ],
            connections: vec![],
            metadata: Metadata { name: None, description: None, source: None },
        };
        let result = gov.validate(netlist);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| matches!(e, PhysicsError::InvalidNetlist(_))),
            "Should reject resistor with 3 pins: {:?}",
            errors
        );
    }
}
