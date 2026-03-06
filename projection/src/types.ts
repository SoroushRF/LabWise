/**
 * LabWise Core Types
 *
 * TypeScript interfaces matching the Rust kernel's data structures.
 * These are the canonical frontend types — everything flows through these.
 */

// ── Netlist Types (mirrors kernel/src/netlist.rs) ────────────────────

export interface Pin {
  pin_id: string;
  node: string;
}

export interface ElectricalLimits {
  voltage_max?: number;
  current_max?: number;
}

export interface NetlistComponent {
  id: string;
  type: 'resistor' | 'led' | 'capacitor' | 'op_amp' | 'wire' | 'battery' | 'diode' | 'arduino';
  pins: Pin[];
  tolerance: number;
  electrical_limits: ElectricalLimits;
  value?: number;
}

export interface Connection {
  from_node: string;
  to_node: string;
}

export interface Metadata {
  name?: string;
  description?: string;
  source?: string;
}

export interface Netlist {
  components: NetlistComponent[];
  connections: Connection[];
  metadata: Metadata;
  arduinoCode?: string; // New: Holds C++/MicroPython source for Wokwi
}

// ── Solver Result Types (mirrors kernel/src/mna.rs) ──────────────────

export interface SimulationResult {
  status: 'valid' | 'rejected' | 'solved' | 'error';
  node_voltages: Record<string, number>;
  branch_currents: Record<string, number>;
  message: string;
}

export interface GovernanceResult {
  status: 'valid' | 'rejected';
  components?: number;
  nodes?: number;
  errors?: string[];
  message: string;
}

// ── Component Failure Types (P1 — failure detection) ─────────────────

export interface ComponentFailure {
  component_id: string;
  failure_mode: 'blown' | 'overheated' | 'over_voltage';
  actual_value: number;
  max_value: number;
  unit: string;
}
