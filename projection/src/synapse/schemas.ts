/**
 * Zod Schemas — Validates raw Gemini output before it enters the app.
 *
 * This is the "Pydantic layer" — if Gemini returns garbage, we catch it here
 * with clear error messages before it ever touches the Zustand store or Wokwi.
 *
 * The schema mirrors the Wokwi-compatible component types so Gemini's output
 * can be mapped directly to a Wokwi diagram.json in the next phase.
 */

import { z } from 'zod';

// ── Supported Wokwi-compatible component types ──────────────────────
// These are the types Gemini is allowed to output.
// Each maps 1:1 to a Wokwi part type (e.g. "arduino-uno" → "wokwi-arduino-uno").
export const ComponentTypeEnum = z.enum([
  // Microcontrollers
  'arduino-uno',
  'arduino-mega',
  'arduino-nano',
  'esp32-devkit-v1',

  // Passive components
  'resistor',
  'potentiometer',
  'capacitor',

  // LEDs & Display
  'led',
  'rgb-led',
  'neopixel',
  'lcd1602',
  'lcd2004',

  // Input
  'pushbutton',
  'slide-switch',
  'membrane-keypad',
  'ir-receiver',
  'photoresistor-sensor',

  // Output
  'buzzer',
  'servo',
  'stepper-motor',

  // Sensors
  'dht22',
  'pir-sensor',
  'hc-sr04',           // Ultrasonic distance
  'hall-effect-sensor',

  // Power
  'battery',

  // Connectivity
  'wire',
]);

export type ComponentType = z.infer<typeof ComponentTypeEnum>;

// ── Pin Schema ───────────────────────────────────────────────────────
const PinSchema = z.object({
  pin_id: z.string().min(1, 'Pin ID cannot be empty'),
  node: z.string().min(1, 'Node name cannot be empty'),
});

// ── Component Schema ─────────────────────────────────────────────────
const ComponentSchema = z.object({
  id: z.string().min(1, 'Component ID cannot be empty'),
  type: ComponentTypeEnum,
  pins: z.array(PinSchema).min(1, 'Component must have at least one pin'),
  value: z.union([z.string(), z.number()]).optional(),
  attrs: z.record(z.string(), z.union([z.string(), z.number(), z.boolean()])).optional(),
});

// ── Connection Schema ────────────────────────────────────────────────
const ConnectionSchema = z.object({
  from: z.string().min(1),   // e.g. "uno:13" or "r1:1"
  to: z.string().min(1),     // e.g. "led1:A"
  color: z.string().optional(),
});

// ── Top-level Extraction Result ──────────────────────────────────────
export const ExtractionResultSchema = z.object({
  components: z.array(ComponentSchema).min(1, 'Must extract at least one component'),
  connections: z.array(ConnectionSchema),
  code: z.string().optional(),        // Arduino/MicroPython source code
  language: z.enum(['cpp', 'python', 'none']).optional().default('none'),
  summary: z.string().optional(),     // Human-readable description of the circuit
});

export type ExtractionResult = z.infer<typeof ExtractionResultSchema>;

// ── Validation helper ────────────────────────────────────────────────
export function validateExtraction(raw: unknown): {
  success: true;
  data: ExtractionResult;
} | {
  success: false;
  errors: string[];
} {
  const result = ExtractionResultSchema.safeParse(raw);

  if (result.success) {
    // Post-processing: Filter out any components that Gemini added but have no pins
    // (e.g. magnets, breadboards, etc. which aren't simulation-relevant)
    const filteredComponents = result.data.components.filter(c => c.pins && c.pins.length > 0);
    
    return { 
      success: true, 
      data: {
        ...result.data,
        components: filteredComponents
      } 
    };
  }

  // Format Zod errors into human-readable strings (for the refinement loop)
  const errors = result.error.issues.map((issue) => {
    const path = issue.path.join('.');
    return path ? `${path}: ${issue.message}` : issue.message;
  });

  return { success: false, errors };
}
