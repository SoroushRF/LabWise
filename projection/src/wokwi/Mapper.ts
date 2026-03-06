/**
 * Wokwi Mapper — Converts LabWise ExtractionResult to Wokwi diagram.json format.
 * 
 * Wokwi diagram.json structure:
 * {
 *   "version": 1,
 *   "author": "LabWise",
 *   "editor": "wokwi",
 *   "parts": [ { "type": "wokwi-resistor", "id": "r1", "top": 0, "left": 0, "attrs": { "value": "1000" } } ],
 *   "connections": [ [ "partId:pin", "partId:pin", "color", ["routing"] ] ]
 * }
 */

import type { ExtractionResult } from '../synapse/schemas';

export interface WokwiPart {
  type: string;
  id: string;
  top: number;
  left: number;
  attrs: Record<string, any>;
}

export type WokwiConnection = [string, string, string, string[]];

export interface WokwiDiagram {
  version: number;
  author: string;
  editor: string;
  parts: WokwiPart[];
  connections: WokwiConnection[];
}

/**
 * Maps our internal component types to Wokwi part types.
 */
const TYPE_MAP: Record<string, string> = {
  'arduino-uno': 'wokwi-arduino-uno',
  'arduino-mega': 'wokwi-arduino-mega',
  'arduino-nano': 'wokwi-arduino-nano',
  'esp32-devkit-v1': 'wokwi-esp32-devkit-v1',
  'resistor': 'wokwi-resistor',
  'led': 'wokwi-led',
  'rgb-led': 'wokwi-rgb-led',
  'potentiometer': 'wokwi-potentiometer',
  'pushbutton': 'wokwi-pushbutton',
  'lcd1602': 'wokwi-lcd1602',
  'hc-sr04': 'wokwi-hc-sr04',
  'dht22': 'wokwi-dht22',
  'battery': 'wokwi-battery',
  // Add more as needed based on synapse/schemas.ts
};

/**
 * Converts LabWise JSON to Wokwi Diagram JSON.
 */
export function toWokwiDiagram(extraction: ExtractionResult): WokwiDiagram {
  const parts: WokwiPart[] = extraction.components.map((comp, index) => {
    const wokwiType = TYPE_MAP[comp.type] || `wokwi-${comp.type}`;
    
    // Auto-layout in a simple grid for now
    const top = Math.floor(index / 3) * 150;
    const left = (index % 3) * 200;

    const attrs: Record<string, any> = { ...comp.attrs };
    if (comp.value) {
      // Wokwi resistors expect "value" as a string, e.g., "1000"
      attrs.value = String(comp.value);
    }

    return {
      type: wokwiType,
      id: comp.id,
      top,
      left,
      attrs,
    };
  });

  const connections: WokwiConnection[] = extraction.connections.map((conn) => {
    return [
      conn.from,
      conn.to,
      conn.color || 'green',
      [] // No specific routing path defined yet
    ];
  });

  return {
    version: 1,
    author: 'LabWise',
    editor: 'wokwi',
    parts,
    connections,
  };
}
