//! # LabWise Kernel
//!
//! The deterministic physics engine for LabWise. This crate implements:
//! - **Governance**: Pre-validation of circuit netlists against physical laws
//! - **Component Registry**: Universal parameterized component types
//! - **MNA Solver**: Modified Nodal Analysis for DC circuit simulation (Phase 2)
//! - **Netlist**: Circuit netlist schema and parsing
//!
//! ## Philosophy
//!
//! > AI cannot be trusted with physics.
//!
//! All physics validation is deterministic and hard-coded. The AI (Synapse layer)
//! extracts intent, but this engine enforces the laws of physics before anything
//! is rendered in the 3D projection.

pub mod component_library;
pub mod governance;
pub mod mna;
pub mod netlist;

// These modules will be implemented next:
// pub mod stamper;       (Step 2.1.2)
// pub mod solver;        (Step 2.1.3)
// pub mod newton_raphson; (P2)
// pub mod multimeter;     (P1)
// pub mod failure;         (P1)

/// Re-export key types for convenience
pub use component_library::{spec_from_type, ComponentSpec, ComponentValidationError};
pub use governance::{GovernanceManager, PhysicsError, ValidatedNetlist};
pub use mna::{MnaSystem, MnaSolution, SolverError, solve_circuit};
pub use netlist::{Netlist, Component, Connection, Pin, ComponentType};
