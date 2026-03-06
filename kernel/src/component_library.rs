//! # Universal Component Registry
//!
//! Defines valid component types and their physics-based validation rules.
//! Components are parameterized — ANY valid resistance, capacitance, voltage,
//! etc. is accepted. No hard-coded kit inventories. Physics validation is
//! universal: Ohm's law doesn't change between universities.
//!
//! ## Philosophy
//!
//! The old system hard-coded York University's bookstore kit parts. The new
//! system validates PHYSICS, not INVENTORY. A 2.2kΩ resistor is just as valid
//! as a 330Ω one — as long as the resistance is positive and finite.

use crate::netlist::ComponentType;

/// Specification for a component, derived from its type and value.
/// These are generated dynamically based on the component's parameters,
/// not looked up from a fixed registry.
#[derive(Debug, Clone)]
pub struct ComponentSpec {
    /// Component type
    pub component_type: ComponentType,
    /// Human-readable name (auto-generated from type + value)
    pub name: String,
    /// Maximum voltage rating (volts)
    pub voltage_max: f64,
    /// Maximum current rating (amps)
    pub current_max: f64,
    /// Maximum power dissipation (watts)
    pub power_max: f64,
    /// Number of pins expected
    pub pin_count: usize,
    /// Expected pin names
    pub pin_names: Vec<String>,
}

/// Errors when a component's parameters are physically invalid.
#[derive(Debug, Clone)]
pub enum ComponentValidationError {
    /// Value is negative or zero when it must be positive
    InvalidValue { component_id: String, message: String },
    /// Value exceeds reasonable physical limits
    UnreasonableValue { component_id: String, message: String },
}

impl std::fmt::Display for ComponentValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComponentValidationError::InvalidValue { component_id, message } => {
                write!(f, "INVALID VALUE [{}]: {}", component_id, message)
            }
            ComponentValidationError::UnreasonableValue { component_id, message } => {
                write!(f, "UNREASONABLE VALUE [{}]: {}", component_id, message)
            }
        }
    }
}

/// Generate a ComponentSpec from a component type and its value.
/// This replaces the old hard-coded library lookup with universal physics-based rules.
pub fn spec_from_type(
    component_type: ComponentType,
    value: Option<f64>,
    component_id: &str,
) -> Result<ComponentSpec, ComponentValidationError> {
    match component_type {
        ComponentType::Resistor => {
            let ohms = value.ok_or_else(|| ComponentValidationError::InvalidValue {
                component_id: component_id.to_string(),
                message: "Resistor must have a resistance value (ohms)".to_string(),
            })?;
            if ohms <= 0.0 {
                return Err(ComponentValidationError::InvalidValue {
                    component_id: component_id.to_string(),
                    message: format!("Resistance must be positive, got {:.2}Ω", ohms),
                });
            }
            if ohms > 100e6 {
                return Err(ComponentValidationError::UnreasonableValue {
                    component_id: component_id.to_string(),
                    message: format!("Resistance {:.0}Ω exceeds 100MΩ — likely an error", ohms),
                });
            }
            // Standard 1/4W resistor limits
            let power_max = 0.25;
            let voltage_max = (power_max * ohms).sqrt().min(200.0);
            let current_max = (power_max / ohms).sqrt();

            Ok(ComponentSpec {
                component_type,
                name: format_resistance(ohms),
                voltage_max,
                current_max,
                power_max,
                pin_count: 2,
                pin_names: vec!["pin1".to_string(), "pin2".to_string()],
            })
        }

        ComponentType::Led => {
            // LEDs don't need a resistance value — their forward voltage and
            // max current are standardized across typical lab LEDs.
            Ok(ComponentSpec {
                component_type,
                name: "LED".to_string(),
                voltage_max: 3.5,    // covers most LED colors
                current_max: 0.02,   // 20mA standard
                power_max: 0.07,     // ~70mW
                pin_count: 2,
                pin_names: vec!["anode".to_string(), "cathode".to_string()],
            })
        }

        ComponentType::Capacitor => {
            let farads = value.ok_or_else(|| ComponentValidationError::InvalidValue {
                component_id: component_id.to_string(),
                message: "Capacitor must have a capacitance value (farads)".to_string(),
            })?;
            if farads <= 0.0 {
                return Err(ComponentValidationError::InvalidValue {
                    component_id: component_id.to_string(),
                    message: format!("Capacitance must be positive, got {:.2e}F", farads),
                });
            }
            if farads > 1.0 {
                return Err(ComponentValidationError::UnreasonableValue {
                    component_id: component_id.to_string(),
                    message: format!("Capacitance {:.2e}F exceeds 1F — likely an error", farads),
                });
            }

            Ok(ComponentSpec {
                component_type,
                name: format_capacitance(farads),
                voltage_max: 50.0,   // conservative default
                current_max: 1.0,
                power_max: 0.5,
                pin_count: 2,
                pin_names: vec!["pin1".to_string(), "pin2".to_string()],
            })
        }

        ComponentType::Diode => {
            Ok(ComponentSpec {
                component_type,
                name: "Diode".to_string(),
                voltage_max: 75.0,   // reverse voltage
                current_max: 0.3,    // 300mA forward
                power_max: 0.5,
                pin_count: 2,
                pin_names: vec!["anode".to_string(), "cathode".to_string()],
            })
        }

        ComponentType::Battery => {
            let volts = value.ok_or_else(|| ComponentValidationError::InvalidValue {
                component_id: component_id.to_string(),
                message: "Battery/voltage source must have a voltage value".to_string(),
            })?;
            if volts <= 0.0 {
                return Err(ComponentValidationError::InvalidValue {
                    component_id: component_id.to_string(),
                    message: format!("Voltage must be positive, got {:.2}V", volts),
                });
            }
            if volts > 48.0 {
                return Err(ComponentValidationError::UnreasonableValue {
                    component_id: component_id.to_string(),
                    message: format!("{:.1}V exceeds safe lab voltage (48V max)", volts),
                });
            }

            Ok(ComponentSpec {
                component_type,
                name: format!("{:.1}V Source", volts),
                voltage_max: volts * 1.1, // 10% headroom
                current_max: 0.5,
                power_max: volts * 0.5,
                pin_count: 2,
                pin_names: vec!["positive".to_string(), "negative".to_string()],
            })
        }

        ComponentType::Wire => {
            Ok(ComponentSpec {
                component_type,
                name: "Jumper Wire".to_string(),
                voltage_max: 300.0,
                current_max: 3.0,
                power_max: 1.0,
                pin_count: 2,
                pin_names: vec!["pin1".to_string(), "pin2".to_string()],
            })
        }

        ComponentType::OpAmp => {
            Ok(ComponentSpec {
                component_type,
                name: "Op-Amp".to_string(),
                voltage_max: 32.0,
                current_max: 0.02,
                power_max: 0.83,
                pin_count: 8,
                pin_names: vec![
                    "out_a".to_string(),
                    "in-_a".to_string(),
                    "in+_a".to_string(),
                    "vee".to_string(),
                    "in+_b".to_string(),
                    "in-_b".to_string(),
                    "out_b".to_string(),
                    "vcc".to_string(),
                ],
            })
        }
    }
}

/// Format a resistance value in human-readable form (e.g., "1kΩ", "330Ω", "4.7MΩ")
fn format_resistance(ohms: f64) -> String {
    if ohms >= 1e6 {
        format!("{:.1}MΩ Resistor", ohms / 1e6)
    } else if ohms >= 1e3 {
        format!("{:.1}kΩ Resistor", ohms / 1e3)
    } else {
        format!("{:.0}Ω Resistor", ohms)
    }
}

/// Format a capacitance value in human-readable form (e.g., "100nF", "10µF")
fn format_capacitance(farads: f64) -> String {
    if farads >= 1e-3 {
        format!("{:.1}mF Capacitor", farads * 1e3)
    } else if farads >= 1e-6 {
        format!("{:.1}µF Capacitor", farads * 1e6)
    } else if farads >= 1e-9 {
        format!("{:.0}nF Capacitor", farads * 1e9)
    } else {
        format!("{:.0}pF Capacitor", farads * 1e12)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resistor_any_value() {
        // Any positive resistance should be accepted
        let values = [1.0, 100.0, 330.0, 1000.0, 2200.0, 4700.0, 10_000.0, 1e6];
        for ohms in values {
            let spec = spec_from_type(ComponentType::Resistor, Some(ohms), "R1");
            assert!(spec.is_ok(), "Resistor {:.0}Ω should be valid", ohms);
            let spec = spec.unwrap();
            assert_eq!(spec.pin_count, 2);
            assert!(spec.voltage_max > 0.0);
            assert!(spec.current_max > 0.0);
        }
    }

    #[test]
    fn test_resistor_zero_rejected() {
        let spec = spec_from_type(ComponentType::Resistor, Some(0.0), "R1");
        assert!(spec.is_err(), "0Ω resistor should be rejected");
    }

    #[test]
    fn test_resistor_negative_rejected() {
        let spec = spec_from_type(ComponentType::Resistor, Some(-100.0), "R1");
        assert!(spec.is_err(), "Negative resistance should be rejected");
    }

    #[test]
    fn test_resistor_no_value_rejected() {
        let spec = spec_from_type(ComponentType::Resistor, None, "R1");
        assert!(spec.is_err(), "Resistor without value should be rejected");
    }

    #[test]
    fn test_battery_any_voltage() {
        let values = [1.5, 3.0, 3.3, 5.0, 9.0, 12.0, 24.0];
        for volts in values {
            let spec = spec_from_type(ComponentType::Battery, Some(volts), "V1");
            assert!(spec.is_ok(), "Battery {:.1}V should be valid", volts);
        }
    }

    #[test]
    fn test_battery_unsafe_voltage_rejected() {
        let spec = spec_from_type(ComponentType::Battery, Some(120.0), "V1");
        assert!(spec.is_err(), "120V should be rejected as unsafe for lab");
    }

    #[test]
    fn test_led_no_value_needed() {
        let spec = spec_from_type(ComponentType::Led, None, "LED1");
        assert!(spec.is_ok(), "LED should not require a value");
        let spec = spec.unwrap();
        assert_eq!(spec.current_max, 0.02); // 20mA
        assert_eq!(spec.pin_count, 2);
    }

    #[test]
    fn test_capacitor_any_value() {
        let values = [100e-12, 100e-9, 10e-6, 100e-6, 1e-3];
        for farads in values {
            let spec = spec_from_type(ComponentType::Capacitor, Some(farads), "C1");
            assert!(spec.is_ok(), "Capacitor {:.2e}F should be valid", farads);
        }
    }

    #[test]
    fn test_wire_no_value_needed() {
        let spec = spec_from_type(ComponentType::Wire, None, "W1");
        assert!(spec.is_ok());
    }

    #[test]
    fn test_format_resistance() {
        assert_eq!(format_resistance(330.0), "330Ω Resistor");
        assert_eq!(format_resistance(1000.0), "1.0kΩ Resistor");
        assert_eq!(format_resistance(4700.0), "4.7kΩ Resistor");
        assert_eq!(format_resistance(1e6), "1.0MΩ Resistor");
    }

    #[test]
    fn test_format_capacitance() {
        assert_eq!(format_capacitance(100e-9), "100nF Capacitor");
        assert_eq!(format_capacitance(10e-6), "10.0µF Capacitor");
        assert_eq!(format_capacitance(100e-12), "100pF Capacitor");
    }
}
