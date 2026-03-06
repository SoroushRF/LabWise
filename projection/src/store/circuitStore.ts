/**
 * Circuit Store — Zustand state management for circuit data
 *
 * Holds the netlist, simulation solution, and component failures.
 * When the netlist changes, the solver runs automatically via WASM.
 */

import { create } from 'zustand';
import type {
  Netlist,
  SimulationResult,
  ComponentFailure,
  GovernanceResult,
} from '../types';

interface CircuitState {
  // ── Data ────────────────────────────────────────────────────────
  netlist: Netlist | null;
  arduinoCode: string | null;
  solution: SimulationResult | null;
  governanceResult: GovernanceResult | null;
  failures: ComponentFailure[];
  isLoading: boolean;
  error: string | null;

  // ── Actions ─────────────────────────────────────────────────────
  setNetlist: (netlist: Netlist) => void;
  setArduinoCode: (code: string) => void;
  solve: () => Promise<void>;
  loadSampleCircuit: (name: string) => void;
  clear: () => void;
}

// Sample circuits — pre-loaded for the demo
const SAMPLE_CIRCUITS: Record<string, Netlist> = {
  // ... (previous samples kept)
  'arduino-blink': {
    components: [
      {
        id: 'MCU1',
        type: 'arduino',
        pins: [
          { pin_id: '13', node: 'd13' },
          { pin_id: 'GND', node: 'GND' },
        ],
        tolerance: 0,
        electrical_limits: { voltage_max: 5, current_max: 0.04 },
      },
      {
        id: 'LED1',
        type: 'led',
        pins: [
          { pin_id: 'anode', node: 'd13' },
          { pin_id: 'cathode', node: 'GND' },
        ],
        tolerance: 0.1,
        electrical_limits: { voltage_max: 2.2, current_max: 0.02 },
      },
    ],
    connections: [],
    arduinoCode: `void setup() {
  pinMode(13, OUTPUT);
}

void loop() {
  digitalWrite(13, HIGH);
  delay(500);
  digitalWrite(13, LOW);
  delay(500);
}`,
    metadata: {
      name: 'Arduino Blink',
      description: 'Standard Arduino Uno blink example with an external LED',
    },
  },
};

export const useCircuitStore = create<CircuitState>((set, get) => ({
  netlist: null,
  arduinoCode: null,
  solution: null,
  governanceResult: null,
  failures: [],
  isLoading: false,
  error: null,

  setNetlist: (netlist) => {
    set({ 
      netlist, 
      arduinoCode: netlist.arduinoCode || null,
      error: null 
    });
    get().solve();
  },

  setArduinoCode: (code) => set({ arduinoCode: code }),

  solve: async () => {
    const { netlist } = get();
    if (!netlist) return;

    set({ isLoading: true, error: null });

    try {
      // Import the WASM module dynamically
      const wasm = await import('../wasm-pkg/labwise_bridge.js');
      await wasm.default();

      // Convert netlist types to match the Rust kernel's expected format
      const kernelNetlist = {
        components: netlist.components.map((c) => ({
          id: c.id,
          component_type: c.type,
          pins: c.pins,
          tolerance: c.tolerance,
          electrical_limits: c.electrical_limits,
          value: c.value ?? null,
        })),
        connections: netlist.connections,
        metadata: netlist.metadata,
      };

      // Call the WASM validate + solve function
      const resultJson = wasm.validate_circuit(JSON.stringify(kernelNetlist));
      const result = JSON.parse(resultJson);

      if (result.status === 'valid' || result.status === 'solved') {
        set({
          governanceResult: {
            status: 'valid',
            components: result.components,
            nodes: result.nodes,
            message: result.message,
          },
          solution: result.solution ?? null,
          isLoading: false,
        });
      } else {
        set({
          governanceResult: {
            status: 'rejected',
            errors: result.errors,
            message: result.message,
          },
          solution: null,
          isLoading: false,
        });
      }
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : 'Unknown solver error',
        isLoading: false,
      });
    }
  },

  loadSampleCircuit: (name) => {
    const circuit = SAMPLE_CIRCUITS[name];
    if (circuit) {
      get().setNetlist(circuit);
    }
  },

  clear: () =>
    set({
      netlist: null,
      arduinoCode: null,
      solution: null,
      governanceResult: null,
      failures: [],
      error: null,
    }),
}));

// Export sample circuit names for the UI
export const SAMPLE_CIRCUIT_NAMES = Object.keys(SAMPLE_CIRCUITS);
