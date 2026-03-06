//! # Modified Nodal Analysis (MNA) System
//!
//! Implements the core MNA matrix assembly for DC circuit simulation.
//! The system transforms a circuit netlist into the linear equation **Ax = z**:
//!
//! ```text
//! | G  B | | v |   | i_s |
//! |      | |   | = |     |
//! | C  D | | i |   | v_s |
//! ```
//!
//! Where:
//! - **G** (conductance matrix): NxN, built from resistor stamps
//! - **B** (voltage source coupling): NxM, links voltage sources to nodes
//! - **C** (current measurement): MxN, transpose relationship to B
//! - **D** (dependent sources): MxM, zero for independent sources
//! - **v** (unknown node voltages): Nx1
//! - **i** (unknown branch currents through voltage sources): Mx1
//! - **i_s** (known current source injections): Nx1
//! - **v_s** (known voltage source values): Mx1
//!
//! N = number of nodes (excluding ground/node 0)
//! M = number of voltage sources (batteries)

use nalgebra::DMatrix;
use std::collections::HashMap;

/// The MNA system: encapsulates the A matrix and z vector for Ax = z.
#[derive(Debug, Clone)]
pub struct MnaSystem {
    /// Number of nodes (excluding ground, which is node 0)
    pub num_nodes: usize,
    /// Number of voltage sources (batteries)
    pub num_vsources: usize,
    /// Total system size (num_nodes + num_vsources)
    pub total_size: usize,
    /// The A matrix: [G B; C D]
    pub a_matrix: DMatrix<f64>,
    /// The z vector (right-hand side): [i_s; v_s]
    pub z_vector: DMatrix<f64>,
    /// Maps node name (string) to node index (1-indexed internally, 0 = ground)
    pub node_map: HashMap<String, usize>,
    /// Maps voltage source component ID to its index (0-indexed)
    pub vsource_map: HashMap<String, usize>,
}

/// The solution to an MNA system.
#[derive(Debug, Clone)]
pub struct MnaSolution {
    /// Node voltages indexed by node name
    pub node_voltages: HashMap<String, f64>,
    /// Branch currents through voltage sources, indexed by source ID
    pub branch_currents: HashMap<String, f64>,
}

/// Errors that can occur during MNA solving.
#[derive(Debug, Clone)]
pub enum SolverError {
    /// The A matrix is singular (cannot be inverted)
    SingularMatrix,
    /// A node referenced in the circuit was not found
    NodeNotFound(String),
    /// The system has no equations to solve
    EmptySystem,
    /// Newton-Raphson did not converge (P2)
    DidNotConverge,
    /// Time budget exceeded (P2)
    TimeBudgetExceeded,
}

impl std::fmt::Display for SolverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SolverError::SingularMatrix => write!(f, "Singular matrix: circuit may have floating nodes or redundant equations"),
            SolverError::NodeNotFound(n) => write!(f, "Node '{}' not found in the system", n),
            SolverError::EmptySystem => write!(f, "MNA system has no equations to solve"),
            SolverError::DidNotConverge => write!(f, "Newton-Raphson solver did not converge"),
            SolverError::TimeBudgetExceeded => write!(f, "Solver exceeded time budget"),
        }
    }
}

impl MnaSystem {
    /// Create a new empty MNA system from a validated netlist.
    ///
    /// This scans the netlist to:
    /// 1. Assign each unique node (except ground) a numeric index
    /// 2. Count voltage sources (batteries)
    /// 3. Allocate the A matrix and z vector with the correct dimensions
    pub fn from_validated(validated: &crate::governance::ValidatedNetlist) -> Self {
        let netlist = &validated.netlist;

        // Step 1: Build node map — assign indices to all non-ground nodes
        let mut node_map = HashMap::new();
        let mut node_index = 0usize;

        for component in &netlist.components {
            for pin in &component.pins {
                let lower = pin.node.to_lowercase();
                if lower == "gnd" || lower == "ground" || lower == "0" {
                    continue; // Ground is node 0, not in the matrix
                }
                if !node_map.contains_key(&pin.node) {
                    node_index += 1;
                    node_map.insert(pin.node.clone(), node_index);
                }
            }
        }

        // Also scan explicit connections
        for conn in &netlist.connections {
            for node_name in [&conn.from_node, &conn.to_node] {
                let lower = node_name.to_lowercase();
                if lower == "gnd" || lower == "ground" || lower == "0" {
                    continue;
                }
                if !node_map.contains_key(node_name) {
                    node_index += 1;
                    node_map.insert(node_name.clone(), node_index);
                }
            }
        }

        let num_nodes = node_index;

        // Step 2: Count and map voltage sources
        let mut vsource_map = HashMap::new();
        let mut vsource_index = 0usize;

        for component in &netlist.components {
            if component.component_type == crate::netlist::ComponentType::Battery {
                vsource_map.insert(component.id.clone(), vsource_index);
                vsource_index += 1;
            }
        }

        let num_vsources = vsource_index;
        let total_size = num_nodes + num_vsources;

        // Step 3: Allocate matrices
        let a_matrix = DMatrix::zeros(total_size, total_size);
        let z_vector = DMatrix::zeros(total_size, 1);

        MnaSystem {
            num_nodes,
            num_vsources,
            total_size,
            a_matrix,
            z_vector,
            node_map,
            vsource_map,
        }
    }

    /// Get the matrix index for a node name. Returns 0 for ground nodes.
    /// Returns None if the node is not found.
    pub fn node_index(&self, node_name: &str) -> Option<usize> {
        let lower = node_name.to_lowercase();
        if lower == "gnd" || lower == "ground" || lower == "0" {
            Some(0) // Ground
        } else {
            self.node_map.get(node_name).copied()
        }
    }

    /// Check if a node name is ground.
    pub fn is_ground(node_name: &str) -> bool {
        let lower = node_name.to_lowercase();
        lower == "gnd" || lower == "ground" || lower == "0"
    }

    // ── Stamper Methods (Step 2.1.2) ────────────────────────────────

    /// Stamp a resistor between two nodes.
    ///
    /// Conductance g = 1/R. The stamp modifies the G sub-matrix:
    /// ```text
    /// G[n1][n1] += g
    /// G[n2][n2] += g
    /// G[n1][n2] -= g
    /// G[n2][n1] -= g
    /// ```
    /// If either node is ground (index 0), those entries are skipped.
    pub fn stamp_resistor(&mut self, conductance: f64, node1: usize, node2: usize) {
        // node indices are 1-indexed in node_map, but 0-indexed in the matrix
        // Matrix row/col = node_index - 1 (since ground=0 is excluded)
        if node1 != 0 {
            let r = node1 - 1;
            self.a_matrix[(r, r)] += conductance;
        }
        if node2 != 0 {
            let r = node2 - 1;
            self.a_matrix[(r, r)] += conductance;
        }
        if node1 != 0 && node2 != 0 {
            let r1 = node1 - 1;
            let r2 = node2 - 1;
            self.a_matrix[(r1, r2)] -= conductance;
            self.a_matrix[(r2, r1)] -= conductance;
        }
    }

    /// Stamp a voltage source between two nodes.
    ///
    /// Voltage sources add extra rows/columns to the MNA system.
    /// The stamp modifies the B, C sub-matrices and z vector:
    /// ```text
    /// B[pos][idx] += 1,  B[neg][idx] -= 1
    /// C[idx][pos] += 1,  C[idx][neg] -= 1
    /// z[num_nodes + idx] = voltage
    /// ```
    pub fn stamp_voltage_source(&mut self, voltage: f64, pos: usize, neg: usize, vs_idx: usize) {
        let row = self.num_nodes + vs_idx; // Row in the augmented system

        // B sub-matrix (upper-right): columns start at num_nodes
        if pos != 0 {
            let p = pos - 1;
            self.a_matrix[(p, row)] += 1.0;
        }
        if neg != 0 {
            let n = neg - 1;
            self.a_matrix[(n, row)] -= 1.0;
        }

        // C sub-matrix (lower-left): rows start at num_nodes
        if pos != 0 {
            let p = pos - 1;
            self.a_matrix[(row, p)] += 1.0;
        }
        if neg != 0 {
            let n = neg - 1;
            self.a_matrix[(row, n)] -= 1.0;
        }

        // z vector: voltage source value
        self.z_vector[(row, 0)] = voltage;
    }

    /// Stamp a current source between two nodes.
    ///
    /// Current flows from neg to pos (conventional current direction).
    /// ```text
    /// z[pos] += current
    /// z[neg] -= current
    /// ```
    pub fn stamp_current_source(&mut self, current: f64, pos: usize, neg: usize) {
        if pos != 0 {
            let p = pos - 1;
            self.z_vector[(p, 0)] += current;
        }
        if neg != 0 {
            let n = neg - 1;
            self.z_vector[(n, 0)] -= current;
        }
    }

    /// Stamp an entire validated circuit into the MNA system.
    ///
    /// Iterates all components and stamps each one:
    /// - Resistors: stamp as conductance (1/R)
    /// - Batteries: stamp as voltage source
    /// - LEDs/Diodes: modeled as a fixed voltage drop for the linear P0 solver
    ///   (Newton-Raphson non-linear model is P2)
    /// - Wires: stamp as very high conductance (1e9 S)
    /// - Capacitors: treated as open circuit in DC (no stamp)
    pub fn stamp_circuit(&mut self, validated: &crate::governance::ValidatedNetlist) {
        use crate::netlist::ComponentType;

        for component in &validated.netlist.components {
            let pin_nodes: Vec<usize> = component
                .pins
                .iter()
                .map(|pin| self.node_index(&pin.node).unwrap_or(0))
                .collect();

            match component.component_type {
                ComponentType::Resistor => {
                    if pin_nodes.len() == 2 {
                        let r = component.value.unwrap_or(1000.0);
                        let g = 1.0 / r; // conductance = 1/resistance
                        self.stamp_resistor(g, pin_nodes[0], pin_nodes[1]);
                    }
                }

                ComponentType::Battery => {
                    if pin_nodes.len() == 2 {
                        let voltage = component.value.unwrap_or(0.0);
                        let vs_idx = self.vsource_map[&component.id];
                        // Positive terminal is pin[0], negative is pin[1]
                        self.stamp_voltage_source(voltage, pin_nodes[0], pin_nodes[1], vs_idx);
                    }
                }

                ComponentType::Led | ComponentType::Diode => {
                    // P0 linear model: LED = fixed 2V voltage drop (anode to cathode)
                    // This uses an additional voltage source entry.
                    // For the P0 MVP, we model LEDs as resistors with a typical
                    // operating resistance. At 20mA with ~2V drop: R ≈ 100Ω
                    if pin_nodes.len() == 2 {
                        let led_resistance = 100.0; // Simplified linear model
                        let g = 1.0 / led_resistance;
                        self.stamp_resistor(g, pin_nodes[0], pin_nodes[1]);
                    }
                }

                ComponentType::Wire => {
                    // Wire = very high conductance (very low resistance)
                    if pin_nodes.len() == 2 {
                        let g = 1e9; // ~0Ω
                        self.stamp_resistor(g, pin_nodes[0], pin_nodes[1]);
                    }
                }

                ComponentType::Capacitor => {
                    // DC analysis: capacitor = open circuit (no stamp)
                }

                ComponentType::OpAmp => {
                    // Op-amp stamping is P2 — skip for now
                }
            }
        }
    }

    // ── Solver (Step 2.1.3) ─────────────────────────────────────────

    /// Solve the MNA system using LU decomposition.
    ///
    /// Returns node voltages and branch currents, or a SolverError
    /// if the matrix is singular or the system is empty.
    pub fn solve(&self) -> Result<MnaSolution, SolverError> {
        if self.total_size == 0 {
            return Err(SolverError::EmptySystem);
        }

        // Use nalgebra's LU decomposition to solve Ax = z
        let lu = self.a_matrix.clone().lu();
        let x = lu.solve(&self.z_vector).ok_or(SolverError::SingularMatrix)?;

        // Extract node voltages (first num_nodes entries)
        let mut node_voltages = HashMap::new();
        for (name, &idx) in &self.node_map {
            let matrix_row = idx - 1; // node_map is 1-indexed, matrix is 0-indexed
            node_voltages.insert(name.clone(), x[(matrix_row, 0)]);
        }
        // Ground is always 0V
        node_voltages.insert("GND".to_string(), 0.0);

        // Extract branch currents (remaining entries after node voltages)
        let mut branch_currents = HashMap::new();
        for (name, &idx) in &self.vsource_map {
            let matrix_row = self.num_nodes + idx;
            branch_currents.insert(name.clone(), x[(matrix_row, 0)]);
        }

        Ok(MnaSolution {
            node_voltages,
            branch_currents,
        })
    }
}

/// Convenience function: validate, build, stamp, and solve a circuit in one call.
///
/// This is the main entry point for the physics engine.
/// Takes a validated netlist and returns the solved electrical state.
pub fn solve_circuit(validated: &crate::governance::ValidatedNetlist) -> Result<MnaSolution, SolverError> {
    let mut system = MnaSystem::from_validated(validated);
    system.stamp_circuit(validated);
    system.solve()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governance::GovernanceManager;
    use crate::netlist::*;

    fn make_led_circuit() -> Netlist {
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
                name: Some("LED Circuit".to_string()),
                description: None,
                source: None,
            },
        }
    }

    fn make_series_resistor_circuit() -> Netlist {
        // 9V battery + R1(1k) + R2(2k) in series
        // Nodes: VCC, n1, GND
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
                    value: Some(1000.0),
                },
                Component {
                    id: "R2".to_string(),
                    component_type: ComponentType::Resistor,
                    pins: vec![
                        Pin { pin_id: "pin1".to_string(), node: "n1".to_string() },
                        Pin { pin_id: "pin2".to_string(), node: "GND".to_string() },
                    ],
                    tolerance: 0.05,
                    electrical_limits: ElectricalLimits {
                        voltage_max: Some(200.0),
                        current_max: Some(0.25),
                    },
                    value: Some(2000.0),
                },
            ],
            connections: vec![],
            metadata: Metadata {
                name: Some("Voltage Divider".to_string()),
                description: None,
                source: None,
            },
        }
    }

    #[test]
    fn test_mna_system_creation() {
        let gov = GovernanceManager::new();
        let validated = gov.validate(make_led_circuit()).unwrap();
        let mna = MnaSystem::from_validated(&validated);

        // Should have 2 non-ground nodes: VCC and n1
        assert_eq!(mna.num_nodes, 2, "Expected 2 nodes (VCC, n1)");
        // Should have 1 voltage source: BAT1
        assert_eq!(mna.num_vsources, 1, "Expected 1 voltage source");
        // Total size = 2 + 1 = 3
        assert_eq!(mna.total_size, 3);
        // A matrix should be 3x3
        assert_eq!(mna.a_matrix.nrows(), 3);
        assert_eq!(mna.a_matrix.ncols(), 3);
        // z vector should be 3x1
        assert_eq!(mna.z_vector.nrows(), 3);
    }

    #[test]
    fn test_node_mapping() {
        let gov = GovernanceManager::new();
        let validated = gov.validate(make_led_circuit()).unwrap();
        let mna = MnaSystem::from_validated(&validated);

        // VCC and n1 should be mapped to indices 1 and 2 (order depends on iteration)
        assert!(mna.node_map.contains_key("VCC"));
        assert!(mna.node_map.contains_key("n1"));
        assert!(!mna.node_map.contains_key("GND")); // Ground is not in the map

        // Ground should return index 0
        assert_eq!(mna.node_index("GND"), Some(0));
        assert_eq!(mna.node_index("gnd"), Some(0));
        assert_eq!(mna.node_index("ground"), Some(0));

        // VCC should return a valid index
        let vcc_idx = mna.node_index("VCC").unwrap();
        assert!(vcc_idx >= 1 && vcc_idx <= 2);
    }

    #[test]
    fn test_vsource_mapping() {
        let gov = GovernanceManager::new();
        let validated = gov.validate(make_series_resistor_circuit()).unwrap();
        let mna = MnaSystem::from_validated(&validated);

        assert_eq!(mna.vsource_map.len(), 1);
        assert!(mna.vsource_map.contains_key("BAT1"));
        assert_eq!(mna.vsource_map["BAT1"], 0);
    }

    #[test]
    fn test_series_circuit_dimensions() {
        let gov = GovernanceManager::new();
        let validated = gov.validate(make_series_resistor_circuit()).unwrap();
        let mna = MnaSystem::from_validated(&validated);

        // 9V + R1(1k) + R2(2k): nodes VCC, n1 (GND excluded) = 2 nodes
        // 1 voltage source (BAT1)
        // Total = 3x3
        assert_eq!(mna.num_nodes, 2);
        assert_eq!(mna.num_vsources, 1);
        assert_eq!(mna.total_size, 3);
    }

    #[test]
    fn test_ground_detection() {
        assert!(MnaSystem::is_ground("GND"));
        assert!(MnaSystem::is_ground("gnd"));
        assert!(MnaSystem::is_ground("ground"));
        assert!(MnaSystem::is_ground("0"));
        assert!(!MnaSystem::is_ground("VCC"));
        assert!(!MnaSystem::is_ground("n1"));
    }

    // ── Stamper Tests (Step 2.1.2) ──────────────────────────────────

    #[test]
    fn test_stamp_resistor() {
        let gov = GovernanceManager::new();
        let validated = gov.validate(make_series_resistor_circuit()).unwrap();
        let mut mna = MnaSystem::from_validated(&validated);

        // Manually stamp a 1kΩ resistor between node indices 1 and 2
        let g = 1.0 / 1000.0; // 0.001 S
        mna.stamp_resistor(g, 1, 2);

        // Check the conductance stamp pattern
        assert!((mna.a_matrix[(0, 0)] - g).abs() < 1e-12, "G[1][1] should be +g");
        assert!((mna.a_matrix[(1, 1)] - g).abs() < 1e-12, "G[2][2] should be +g");
        assert!((mna.a_matrix[(0, 1)] + g).abs() < 1e-12, "G[1][2] should be -g");
        assert!((mna.a_matrix[(1, 0)] + g).abs() < 1e-12, "G[2][1] should be -g");
    }

    #[test]
    fn test_stamp_resistor_to_ground() {
        let gov = GovernanceManager::new();
        let validated = gov.validate(make_series_resistor_circuit()).unwrap();
        let mut mna = MnaSystem::from_validated(&validated);

        // Stamp a 2kΩ resistor from node 2 to ground (node 0)
        let g = 1.0 / 2000.0;
        mna.stamp_resistor(g, 2, 0);

        // Only G[2][2] should be stamped (ground entries are skipped)
        assert!((mna.a_matrix[(1, 1)] - g).abs() < 1e-12, "G[2][2] should be +g");
    }

    #[test]
    fn test_stamp_voltage_source() {
        let gov = GovernanceManager::new();
        let validated = gov.validate(make_series_resistor_circuit()).unwrap();
        let mut mna = MnaSystem::from_validated(&validated);

        let vcc_idx = mna.node_index("VCC").unwrap();

        // Stamp a 9V source: VCC(+) to GND(-)
        mna.stamp_voltage_source(9.0, vcc_idx, 0, 0);

        let vs_row = mna.num_nodes; // augmented row for vsource 0
        let vcc_col = vcc_idx - 1;  // matrix column for VCC

        // B sub-matrix: a_matrix[vcc_col][vs_row] should be 1.0
        assert!((mna.a_matrix[(vcc_col, vs_row)] - 1.0).abs() < 1e-12, "B[VCC][0] should be 1");
        // C sub-matrix: a_matrix[vs_row][vcc_col] should be 1.0
        assert!((mna.a_matrix[(vs_row, vcc_col)] - 1.0).abs() < 1e-12, "C[0][VCC] should be 1");
        // z vector: voltage
        assert!((mna.z_vector[(vs_row, 0)] - 9.0).abs() < 1e-12, "z[vs_row] should be 9V");
    }

    #[test]
    fn test_stamp_circuit_voltage_divider() {
        let gov = GovernanceManager::new();
        let validated = gov.validate(make_series_resistor_circuit()).unwrap();
        let mut mna = MnaSystem::from_validated(&validated);

        // Auto-stamp the entire circuit
        mna.stamp_circuit(&validated);

        // After stamping, the A matrix should be non-zero
        let nonzero_count = mna.a_matrix.iter().filter(|&&v| v.abs() > 1e-15).count();
        assert!(nonzero_count > 0, "A matrix should have non-zero entries after stamping");

        // The z vector should have the voltage source value
        let vs_row = mna.num_nodes; // row for the voltage source
        assert!((mna.z_vector[(vs_row, 0)] - 9.0).abs() < 1e-12,
            "z vector should contain 9V for the battery");

        // R1 = 1kΩ: conductance = 0.001 S should appear in G sub-matrix
        // R2 = 2kΩ: conductance = 0.0005 S should appear in G sub-matrix
        let vcc_idx = mna.node_index("VCC").unwrap();
        let n1_idx = mna.node_index("n1").unwrap();
        let vcc_r = vcc_idx - 1;
        let n1_r = n1_idx - 1;

        // G[VCC][VCC] should have R1's conductance (1/1000)
        assert!(mna.a_matrix[(vcc_r, vcc_r)] > 0.0,
            "G[VCC][VCC] should be positive (has R1 conductance)");
        // G[n1][n1] should have R1 + R2 conductance
        assert!(mna.a_matrix[(n1_r, n1_r)] > 0.0,
            "G[n1][n1] should be positive (has R1 + R2 conductance)");
    }

    // ── Solver Tests (Step 2.1.3) ───────────────────────────────────

    #[test]
    fn test_solve_voltage_divider() {
        // 9V battery + R1(1kΩ) + R2(2kΩ) in series
        // Hand calculation:
        //   V_VCC = 9V (set by battery)
        //   V_n1 = 9V * R2/(R1+R2) = 9 * 2000/3000 = 6.0V
        //   I = 9V / 3000Ω = 0.003A = 3.0mA
        let gov = GovernanceManager::new();
        let validated = gov.validate(make_series_resistor_circuit()).unwrap();
        let solution = solve_circuit(&validated).unwrap();

        let v_vcc = solution.node_voltages["VCC"];
        let v_n1 = solution.node_voltages["n1"];
        let v_gnd = solution.node_voltages["GND"];

        // VCC should be 9V
        assert!(
            (v_vcc - 9.0).abs() < 0.01,
            "V_VCC should be 9.0V, got {:.4}V", v_vcc
        );
        // n1 should be 6V (voltage divider: 9 * 2k / (1k + 2k))
        assert!(
            (v_n1 - 6.0).abs() < 0.01,
            "V_n1 should be 6.0V, got {:.4}V", v_n1
        );
        // GND should be 0V
        assert!(
            v_gnd.abs() < 0.01,
            "V_GND should be 0V, got {:.4}V", v_gnd
        );

        // Battery current should be -3mA (negative = current flowing out)
        let i_bat = solution.branch_currents["BAT1"];
        assert!(
            (i_bat.abs() - 0.003).abs() < 0.0001,
            "I_BAT1 should be 3mA, got {:.6}A", i_bat
        );
    }

    #[test]
    fn test_solve_single_resistor() {
        // Simplest circuit: 9V battery + 1kΩ resistor
        // Expected: V_VCC = 9V, I = 9mA
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
            metadata: Metadata {
                name: Some("Single Resistor".to_string()),
                description: None,
                source: None,
            },
        };

        let gov = GovernanceManager::new();
        let validated = gov.validate(netlist).unwrap();
        let solution = solve_circuit(&validated).unwrap();

        let v_vcc = solution.node_voltages["VCC"];
        assert!(
            (v_vcc - 9.0).abs() < 0.01,
            "V_VCC should be 9.0V, got {:.4}V", v_vcc
        );

        let i_bat = solution.branch_currents["BAT1"];
        assert!(
            (i_bat.abs() - 0.009).abs() < 0.001,
            "I should be 9mA, got {:.6}A", i_bat
        );
    }

    // ── Step 2.1.4: Theoretical Baseline Test ───────────────────────

    #[test]
    fn test_theoretical_baseline_series_parallel() {
        // Master Plan Step 2.1.4:
        // 9V battery, R1=1kΩ series, R2=2kΩ parallel R3=2kΩ
        // R_parallel = (2000 * 2000) / (2000 + 2000) = 1000Ω
        // R_total = R1 + R_parallel = 1000 + 1000 = 2000Ω
        // I_total = 9V / 2000Ω = 0.0045A = 4.5mA
        // V_node = I_total * R_parallel = 0.0045 * 1000 = 4.5V
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
                Component {
                    id: "R2".to_string(),
                    component_type: ComponentType::Resistor,
                    pins: vec![
                        Pin { pin_id: "pin1".to_string(), node: "n1".to_string() },
                        Pin { pin_id: "pin2".to_string(), node: "GND".to_string() },
                    ],
                    tolerance: 0.05,
                    electrical_limits: ElectricalLimits {
                        voltage_max: Some(200.0),
                        current_max: Some(0.25),
                    },
                    value: Some(2000.0),
                },
                Component {
                    id: "R3".to_string(),
                    component_type: ComponentType::Resistor,
                    pins: vec![
                        Pin { pin_id: "pin1".to_string(), node: "n1".to_string() },
                        Pin { pin_id: "pin2".to_string(), node: "GND".to_string() },
                    ],
                    tolerance: 0.05,
                    electrical_limits: ElectricalLimits {
                        voltage_max: Some(200.0),
                        current_max: Some(0.25),
                    },
                    value: Some(2000.0),
                },
            ],
            connections: vec![],
            metadata: Metadata {
                name: Some("Series-Parallel Baseline".to_string()),
                description: Some("Master Plan Step 2.1.4 theoretical baseline".to_string()),
                source: None,
            },
        };

        let gov = GovernanceManager::new();
        let validated = gov.validate(netlist).unwrap();
        let solution = solve_circuit(&validated).unwrap();

        let v_vcc = solution.node_voltages["VCC"];
        let v_n1 = solution.node_voltages["n1"];
        let i_total = solution.branch_currents["BAT1"].abs();

        // V_VCC = 9V (exact)
        assert!(
            (v_vcc - 9.0).abs() < 0.001,
            "V_VCC should be 9.0V, got {:.6}V", v_vcc
        );

        // V_n1 = 4.5V (within 0.01%)
        let v_n1_error_pct = ((v_n1 - 4.5) / 4.5).abs() * 100.0;
        assert!(
            v_n1_error_pct < 0.01,
            "V_n1 should be 4.5V within 0.01%, got {:.6}V (error: {:.4}%)",
            v_n1, v_n1_error_pct
        );

        // I_total = 4.5mA (within 0.01%)
        let i_expected = 0.0045;
        let i_error_pct = ((i_total - i_expected) / i_expected).abs() * 100.0;
        assert!(
            i_error_pct < 0.01,
            "I_total should be 4.5mA within 0.01%, got {:.6}A (error: {:.4}%)",
            i_total, i_error_pct
        );
    }
}
