//! # Netlist Schema and Parsing
//!
//! Defines the circuit netlist data structures and JSON deserialization.
//! These types mirror the JSON schema defined in `schemas/netlist.schema.json`.

use serde::{Deserialize, Serialize};

/// The top-level netlist representing a complete circuit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Netlist {
    /// All components in the circuit
    pub components: Vec<Component>,
    /// All wire connections between nodes
    pub connections: Vec<Connection>,
    /// Circuit metadata
    pub metadata: Metadata,
}

/// A single circuit component (resistor, LED, capacitor, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    /// Unique identifier for this component (e.g., "R1", "LED1")
    pub id: String,
    /// Component type — must match a known type in ComponentLibrary
    #[serde(rename = "type")]
    pub component_type: ComponentType,
    /// The pins of this component and their node connections
    pub pins: Vec<Pin>,
    /// Manufacturing tolerance (0.0 to 1.0, e.g., 0.05 = 5%)
    pub tolerance: f64,
    /// Electrical limits for safety validation
    pub electrical_limits: ElectricalLimits,
    /// Component value (e.g., resistance in ohms, capacitance in farads)
    #[serde(default)]
    pub value: Option<f64>,
}

/// Supported component types in the LabWise system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentType {
    Resistor,
    Led,
    Capacitor,
    OpAmp,
    Wire,
    Battery,
    Diode,
}

impl std::fmt::Display for ComponentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ComponentType::Resistor => "resistor",
            ComponentType::Led => "led",
            ComponentType::Capacitor => "capacitor",
            ComponentType::OpAmp => "op_amp",
            ComponentType::Wire => "wire",
            ComponentType::Battery => "battery",
            ComponentType::Diode => "diode",
        };
        write!(f, "{}", s)
    }
}

/// A pin on a component, connected to a circuit node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pin {
    /// Pin identifier (e.g., "anode", "cathode", "pin1", "pin2")
    pub pin_id: String,
    /// The circuit node this pin connects to
    pub node: String,
}

/// Electrical limits for a component used by the failure detection system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElectricalLimits {
    /// Maximum voltage across the component (in volts)
    #[serde(default)]
    pub voltage_max: Option<f64>,
    /// Maximum current through the component (in amps)
    #[serde(default)]
    pub current_max: Option<f64>,
}

/// A connection between two nodes in the circuit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    /// Source node identifier
    pub from_node: String,
    /// Destination node identifier
    pub to_node: String,
}

/// Circuit metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    /// Human-readable name for this circuit
    #[serde(default)]
    pub name: Option<String>,
    /// Description of what this circuit does
    #[serde(default)]
    pub description: Option<String>,
    /// Source of the circuit (e.g., "EECS 1011 Lab 3")
    #[serde(default)]
    pub source: Option<String>,
}

impl Netlist {
    /// Parse a netlist from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize the netlist to a JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_netlist() {
        let json = r#"{
            "components": [
                {
                    "id": "R1",
                    "type": "resistor",
                    "pins": [
                        { "pin_id": "pin1", "node": "n1" },
                        { "pin_id": "pin2", "node": "n2" }
                    ],
                    "tolerance": 0.05,
                    "electrical_limits": {
                        "voltage_max": 50.0,
                        "current_max": 0.25
                    },
                    "value": 1000.0
                }
            ],
            "connections": [
                { "from_node": "n1", "to_node": "VCC" },
                { "from_node": "n2", "to_node": "GND" }
            ],
            "metadata": {
                "name": "Simple Resistor Circuit",
                "description": "A single resistor between VCC and GND"
            }
        }"#;

        let netlist = Netlist::from_json(json).expect("Should parse valid netlist");
        assert_eq!(netlist.components.len(), 1);
        assert_eq!(netlist.components[0].id, "R1");
        assert_eq!(netlist.components[0].component_type, ComponentType::Resistor);
        assert_eq!(netlist.components[0].value, Some(1000.0));
        assert_eq!(netlist.connections.len(), 2);
    }

    #[test]
    fn test_roundtrip_serialization() {
        let netlist = Netlist {
            components: vec![Component {
                id: "LED1".to_string(),
                component_type: ComponentType::Led,
                pins: vec![
                    Pin { pin_id: "anode".to_string(), node: "n1".to_string() },
                    Pin { pin_id: "cathode".to_string(), node: "GND".to_string() },
                ],
                tolerance: 0.1,
                electrical_limits: ElectricalLimits {
                    voltage_max: Some(3.3),
                    current_max: Some(0.02),
                },
                value: None,
            }],
            connections: vec![],
            metadata: Metadata {
                name: Some("LED Test".to_string()),
                description: None,
                source: None,
            },
        };

        let json = netlist.to_json().expect("Should serialize");
        let parsed = Netlist::from_json(&json).expect("Should re-parse");
        assert_eq!(parsed.components[0].id, "LED1");
    }
}
