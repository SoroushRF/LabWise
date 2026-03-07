/**
 * CircuitBoard — Renders extracted circuit components using @wokwi/elements
 * (native web components) instead of an iframe.
 *
 * This version supports drag-and-drop for components and real-time wire updates.
 */

import React, { useEffect, useRef, useState, useCallback } from 'react';
import type { WokwiDiagram, WokwiPart } from './Mapper';

// Import wokwi elements — this registers the custom elements globally
import '@wokwi/elements';

/* ------------------------------------------------------------------ */
/*  Type map: wokwi-type string → custom element tag name             */
/* ------------------------------------------------------------------ */
const TAG_MAP: Record<string, string> = {
  'wokwi-arduino-uno': 'wokwi-arduino-uno',
  'wokwi-arduino-mega': 'wokwi-arduino-mega',
  'wokwi-arduino-nano': 'wokwi-arduino-nano',
  'wokwi-esp32-devkit-v1': 'wokwi-esp32-devkit-v1',
  'wokwi-resistor': 'wokwi-resistor',
  'wokwi-led': 'wokwi-led',
  'wokwi-rgb-led': 'wokwi-rgb-led',
  'wokwi-pushbutton': 'wokwi-pushbutton',
  'wokwi-potentiometer': 'wokwi-potentiometer',
  'wokwi-lcd1602': 'wokwi-lcd1602',
  'wokwi-buzzer': 'wokwi-buzzer',
  'wokwi-servo': 'wokwi-servo',
  'wokwi-dht22': 'wokwi-dht22',
  'wokwi-hc-sr04': 'wokwi-hc-sr04',
  'wokwi-vcc': 'wokwi-vcc',
};

/* ------------------------------------------------------------------ */
/*  Pin position cache                                                 */
/* ------------------------------------------------------------------ */
interface PinPos { x: number; y: number }

const getPinPositions = (el: HTMLElement, partType: string): Map<string, PinPos> => {
  const map = new Map<string, PinPos>();
  
  // 1. Check for native Wokwi pinInfo
  const pinInfo = (el as any).pinInfo;
  if (pinInfo && Array.isArray(pinInfo)) {
    for (const pin of pinInfo) {
      map.set(pin.name, { x: pin.x, y: pin.y });
    }
    return map;
  }

  // 2. Fallback for our custom SVGs
  if (partType.includes('vcc') || partType.includes('battery')) {
    map.set('VCC', { x: 5, y: 25 });
    map.set('GND', { x: 75, y: 25 });
    return map;
  }

  return map;
};

/* ------------------------------------------------------------------ */
/*  Wire color palette                                                 */
/* ------------------------------------------------------------------ */
const WIRE_COLORS: Record<string, string> = {
  red: '#ef4444',
  green: '#22c55e',
  blue: '#3b82f6',
  yellow: '#eab308',
  orange: '#f97316',
  purple: '#a855f7',
  black: '#333',
  white: '#e5e7eb',
};

/* ------------------------------------------------------------------ */
/*  Component                                                          */
/* ------------------------------------------------------------------ */
interface CircuitBoardProps {
  diagram: WokwiDiagram;
}

export const CircuitBoard: React.FC<CircuitBoardProps> = ({ diagram }) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const [wires, setWires] = useState<{ x1: number; y1: number; x2: number; y2: number; color: string }[]>([]);
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 50, y: 50 });
  
  // Component positions state for local manipulation
  const [partPositions, setPartPositions] = useState<Record<string, { top: number; left: number }>>({});
  
  // Drag state
  const dragInfo = useRef<{ partId: string; startX: number; startY: number; initialTop: number; initialLeft: number } | null>(null);
  const isPanning = useRef(false);
  const lastMouse = useRef({ x: 0, y: 0 });

  // Initialize positions from diagram
  useEffect(() => {
    const initialPos: Record<string, { top: number; left: number }> = {};
    diagram.parts.forEach(p => {
      initialPos[p.id] = { top: p.top, left: p.left };
    });
    setPartPositions(initialPos);
  }, [diagram]);

  // Compute wire paths based on current component positions
  const computeWires = useCallback(() => {
    if (!containerRef.current) return;

    const wireData: { x1: number; y1: number; x2: number; y2: number; color: string }[] = [];

    for (const conn of diagram.connections) {
      const fromPartId = conn[0].split(':')[0];
      const fromPinName = conn[0].split(':')[1];
      const toPartId = conn[1].split(':')[0];
      const toPinName = conn[1].split(':')[1];
      const color = conn[2] || 'green';

      const fromPart = diagram.parts.find(p => p.id === fromPartId);
      const toPart = diagram.parts.find(p => p.id === toPartId);
      if (!fromPart || !toPart) continue;

      const fromEl = containerRef.current.querySelector(`[data-part-id="${fromPartId}"]`) as HTMLElement;
      const toEl = containerRef.current.querySelector(`[data-part-id="${toPartId}"]`) as HTMLElement;

      if (!fromEl || !toEl) continue;

      const fromPins = getPinPositions(fromEl, fromPart.type);
      const toPins = getPinPositions(toEl, toPart.type);

      const fromPinPos = fromPins.get(fromPinName);
      const toPinPos = toPins.get(toPinName);

      if (!fromPinPos || !toPinPos) {
        // Log once to help debug if pins are missing
        console.warn(`[CircuitBoard] Could not find pins: ${fromPinName} on ${fromPartId} or ${toPinName} on ${toPartId}`);
        continue;
      }

      const fromPos = partPositions[fromPartId] || { top: 0, left: 0 };
      const toPos = partPositions[toPartId] || { top: 0, left: 0 };

      wireData.push({
        x1: fromPos.left + fromPinPos.x,
        y1: fromPos.top + fromPinPos.y,
        x2: toPos.left + toPinPos.x,
        y2: toPos.top + toPinPos.y,
        color: WIRE_COLORS[color] || color,
      });
    }

    setWires(wireData);
  }, [diagram.parts, diagram.connections, partPositions]);

  // Recompute wires whenever positions change
  useEffect(() => {
    // Small delay to ensure WebComponents have populated their pinInfo
    const timer = setTimeout(computeWires, 100);
    return () => clearTimeout(timer);
  }, [computeWires, partPositions]);

  // Mouse Handlers
  const handleMouseDown = (e: React.MouseEvent) => {
    if (e.button === 1 || (e.button === 0 && e.altKey)) {
      isPanning.current = true;
      lastMouse.current = { x: e.clientX, y: e.clientY };
      e.preventDefault();
      return;
    }
  };

  const handleComponentMouseDown = (e: React.MouseEvent, partId: string) => {
    if (e.button === 0 && !e.altKey) {
      e.stopPropagation();
      const pos = partPositions[partId];
      if (pos) {
        dragInfo.current = {
          partId,
          startX: e.clientX,
          startY: e.clientY,
          initialTop: pos.top,
          initialLeft: pos.left,
        };
      }
    }
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    if (isPanning.current) {
      const dx = e.clientX - lastMouse.current.x;
      const dy = e.clientY - lastMouse.current.y;
      setPan(prev => ({ x: prev.x + dx, y: prev.y + dy }));
      lastMouse.current = { x: e.clientX, y: e.clientY };
    } else if (dragInfo.current) {
      const dx = (e.clientX - dragInfo.current.startX) / zoom;
      const dy = (e.clientY - dragInfo.current.startY) / zoom;
      
      setPartPositions(prev => ({
        ...prev,
        [dragInfo.current!.partId]: {
          top: dragInfo.current!.initialTop + dy,
          left: dragInfo.current!.initialLeft + dx,
        }
      }));
    }
  };

  const handleMouseUp = () => {
    isPanning.current = false;
    dragInfo.current = null;
  };

  const handleWheel = (e: React.WheelEvent) => {
    e.preventDefault();
    const delta = e.deltaY > 0 ? -0.1 : 0.1;
    setZoom(prev => Math.max(0.3, Math.min(3, prev + delta)));
  };

  return (
    <div
      className="circuit-board-container"
      style={{
        width: '100%',
        height: '100%',
        background: '#f5f5f0',
        borderRadius: '12px',
        overflow: 'hidden',
        position: 'relative',
        cursor: isPanning.current ? 'grabbing' : (dragInfo.current ? 'grabbing' : 'grab'),
      }}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
      onMouseLeave={handleMouseUp}
      onWheel={handleWheel}
    >
      {/* Toolbar */}
      <div style={{
        position: 'absolute',
        top: 8,
        right: 8,
        zIndex: 10,
        display: 'flex',
        gap: '4px',
        background: 'rgba(0,0,0,0.7)',
        borderRadius: '8px',
        padding: '4px 8px',
      }}>
        <button onClick={() => setZoom(z => Math.min(3, z + 0.2))} style={zoomBtnStyle}>+</button>
        <span style={{ color: '#fff', fontSize: '0.7rem', padding: '4px', minWidth: '40px', textAlign: 'center' }}>
          {Math.round(zoom * 100)}%
        </span>
        <button onClick={() => setZoom(z => Math.max(0.3, z - 0.2))} style={zoomBtnStyle}>−</button>
        <button onClick={() => { setZoom(1); setPan({ x: 50, y: 50 }); }} style={zoomBtnStyle}>⊙</button>
      </div>

      {/* Canvas */}
      <div
        ref={containerRef}
        className="circuit-canvas"
        style={{
          transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`,
          transformOrigin: '0 0',
          position: 'relative',
          width: '2000px',
          height: '1500px',
        }}
      >
        {/* Wire layer (SVG overlay) */}
        <svg
          style={{
            position: 'absolute',
            top: 0,
            left: 0,
            width: '100%',
            height: '100%',
            pointerEvents: 'none',
            zIndex: 5,
          }}
        >
          {wires.map((w, i) => (
            <line
              key={i}
              x1={w.x1}
              y1={w.y1}
              x2={w.x2}
              y2={w.y2}
              stroke={w.color}
              strokeWidth={3}
              strokeLinecap="round"
              opacity={0.8}
            />
          ))}
        </svg>

        {/* Component layer */}
        {diagram.parts.map((part) => (
          <ComponentWrapper 
            key={part.id} 
            part={part} 
            position={partPositions[part.id] || { top: part.top, left: part.left }}
            onMouseDown={(e) => handleComponentMouseDown(e, part.id)}
          />
        ))}
      </div>
    </div>
  );
};

/* ------------------------------------------------------------------ */
/*  Individual component wrapper                                       */
/* ------------------------------------------------------------------ */
interface ComponentWrapperProps {
  part: WokwiPart;
  position: { top: number; left: number };
  onMouseDown: (e: React.MouseEvent) => void;
}

const ComponentWrapper: React.FC<ComponentWrapperProps> = ({ part, position, onMouseDown }) => {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!ref.current) return;
    ref.current.innerHTML = '';

    const tagName = TAG_MAP[part.type] || part.type;

    if (!customElements.get(tagName)) {
      if (part.type.includes('vcc') || part.type.includes('battery')) {
        ref.current.innerHTML = `
          <svg width="80" height="50" viewBox="0 0 80 50" data-part-id="${part.id}" style="display:block;margin:0;user-select:none;">
            <rect x="10" y="10" width="60" height="30" rx="4" fill="#1a1a1a" stroke="#444" stroke-width="2"/>
            <text x="40" y="30" text-anchor="middle" fill="#4ade80" font-size="12" font-weight="bold" font-family="monospace">5V</text>
            <circle cx="5" cy="25" r="4" fill="#ef4444" />
            <text x="5" y="18" text-anchor="middle" fill="#ef4444" font-size="8" font-weight="bold">+</text>
            <circle cx="75" cy="25" r="4" fill="#666" />
            <text x="75" y="18" text-anchor="middle" fill="#888" font-size="8" font-weight="bold">-</text>
            <text x="40" y="48" text-anchor="middle" fill="#666" font-size="8" font-family="sans-serif">${part.id}</text>
          </svg>
        `;
      } else {
        ref.current.innerHTML = `<div style="background:#333;color:#fff;padding:8px;border-radius:4px;font-size:11px;user-select:none;">${part.id} (${tagName})</div>`;
      }
      return;
    }

    const el = document.createElement(tagName);
    (el as any).dataset.partId = part.id;
    el.style.display = 'block';
    el.style.margin = '0';

    if (part.attrs) {
      for (const [key, value] of Object.entries(part.attrs)) {
        if (key === 'color' && tagName === 'wokwi-led') {
          (el as any).color = String(value);
        } else if (key === 'value' && tagName === 'wokwi-resistor') {
          (el as any).value = String(value);
        } else {
          el.setAttribute(key, String(value));
        }
      }
    }

    if (tagName === 'wokwi-led') {
      (el as any).value = true;
      (el as any).brightness = 1.0;
    }

    ref.current.appendChild(el);
  }, [part]);

  return (
    <div
      ref={ref}
      onMouseDown={onMouseDown}
      className={`component-wrapper ${part.type}`}
      style={{
        position: 'absolute',
        top: position.top,
        left: position.left,
        zIndex: 2,
        cursor: 'grab',
        userSelect: 'none',
        padding: 0,
        margin: 0,
      }}
      title={`${part.id} (${part.type})`}
    />
  );
};

/* ------------------------------------------------------------------ */
/*  Zoom button styling                                                */
/* ------------------------------------------------------------------ */
const zoomBtnStyle: React.CSSProperties = {
  background: 'rgba(255,255,255,0.15)',
  border: 'none',
  color: '#fff',
  width: '28px',
  height: '28px',
  borderRadius: '4px',
  cursor: 'pointer',
  fontSize: '1rem',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
};
