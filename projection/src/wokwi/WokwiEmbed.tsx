/**
 * WokwiEmbed — A React component that embeds the Wokwi simulator.
 * 
 * It uses a hidden form to POST the diagram.json and source code 
 * to Wokwi's embed endpoint.
 */

import React, { useEffect, useRef } from 'react';
import type { WokwiDiagram } from './Mapper';

interface WokwiEmbedProps {
  diagram: WokwiDiagram;
  code: string;
}

export const WokwiEmbed: React.FC<WokwiEmbedProps> = ({ diagram, code }) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const iframeRef = useRef<HTMLIFrameElement>(null);

  useEffect(() => {
    // When diagram or code changes, we need to re-initialize the simulation.
    // Wokwi Embed API via POST:
    // https://docs.wokwi.com/embedding
    
    const form = document.createElement('form');
    form.method = 'POST';
    form.action = 'https://wokwi.com/api/projects/embed';
    form.target = 'wokwi-iframe';

    const diagramInput = document.createElement('input');
    diagramInput.type = 'hidden';
    diagramInput.name = 'diagram';
    diagramInput.value = JSON.stringify(diagram);
    form.appendChild(diagramInput);

    const selectInput = document.createElement('input');
    selectInput.type = 'hidden';
    selectInput.name = 'sketch'; // 'sketch' for Arduino, 'main.py' for ESP32/Python
    selectInput.value = code || '// No code extracted';
    form.appendChild(selectInput);

    document.body.appendChild(form);
    form.submit();
    document.body.removeChild(form);
    
  }, [diagram, code]);

  return (
    <div 
      ref={containerRef} 
      style={{ width: '100%', height: '100%', borderRadius: '8px', overflow: 'hidden', background: '#222' }}
    >
      <iframe
        ref={iframeRef}
        name="wokwi-iframe"
        title="Wokwi Simulation"
        style={{ width: '100%', height: '100%', border: 'none' }}
        allow="usb; serial"
      />
    </div>
  );
};
