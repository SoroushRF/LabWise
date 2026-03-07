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
  'battery': 'wokwi-vcc', // Using VCC as the standard 5V power rail
  'hall-effect-sensor': 'wokwi-analog-joystick', // Placeholder for Hall sensor
};

/**
 * Maps AI pin names to Wokwi-specific pin names.
 */
function mapPin(partType: string, pinId: string): string {
  const pid = pinId.toLowerCase();
  
  if (partType === 'led') {
    if (pid === 'c' || pid === 'cathode' || pid === 'k') return 'C';
    if (pid === 'a' || pid === 'anode') return 'A';
  }
  
  if (partType === 'battery' || partType === 'wokwi-vcc') {
    // Wokwi-vcc and wokwi-battery standard pins
    if (pid === 'vcc' || pid === '5v' || pid === 'positive' || pid === 'pos' || pid === '+') return 'VCC';
    if (pid === 'gnd' || pid === 'negative' || pid === 'neg' || pid === '-') return 'GND';
  }

  return pinId;
}

/**
 * Converts LabWise JSON to Wokwi Diagram JSON.
 */
export function toWokwiDiagram(extraction: ExtractionResult): WokwiDiagram {
  const parts: WokwiPart[] = extraction.components.map((comp, index) => {
    let wokwiType = TYPE_MAP[comp.type] || `wokwi-${comp.type}`;
    
    // Auto-layout in a simple grid for now
    const top = Math.floor(index / 3) * 150;
    const left = (index % 3) * 200;

    const attrs: Record<string, any> = { ...comp.attrs };
    if (comp.type === 'resistor' && comp.value) {
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
    const fromCompId = conn.from.split(':')[0];
    const toCompId = conn.to.split(':')[0];

    const fromComp = extraction.components.find(c => c.id === fromCompId);
    const toComp = extraction.components.find(c => c.id === toCompId);

    const fromPinOriginal = conn.from.split(':')[1];
    const toPinOriginal = conn.to.split(':')[1];

    const mappedFromPin = fromComp ? mapPin(fromComp.type, fromPinOriginal) : fromPinOriginal;
    const mappedToPin = toComp ? mapPin(toComp.type, toPinOriginal) : toPinOriginal;

    return [
      `${fromCompId}:${mappedFromPin}`,
      `${toCompId}:${mappedToPin}`,
      conn.color || 'green',
      []
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
