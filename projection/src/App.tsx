/**
 * LabWise — Main Application
 *
 * Pre-Lab Assistant: AI extraction -> Netlist -> Wokwi execution.
 */

import { useState } from 'react';
import { extractCircuit } from './synapse';
import { useCircuitStore } from './store/circuitStore';
import { toWokwiDiagram, type WokwiDiagram } from './wokwi/Mapper';
import { WokwiEmbed } from './wokwi/WokwiEmbed';
import './App.css';

function App() {
  const [labInstructions, setLabInstructions] = useState('');
  const [status, setStatus] = useState<'idle' | 'loading' | 'success' | 'error'>('idle');
  const [statusMessage, setStatusMessage] = useState('');
  const [wokwiDiagram, setWokwiDiagram] = useState<WokwiDiagram | null>(null);
  const [extractedCode, setExtractedCode] = useState<string>('');
  
  const setArduinoCode = useCircuitStore((state) => state.setArduinoCode);

  const handleExtract = async () => {
    if (!labInstructions.trim()) return;

    setStatus('loading');
    setStatusMessage('Consulting Gemini AI...');
    setWokwiDiagram(null);

    const response = await extractCircuit(labInstructions);

    if (response.status === 'success') {
      setStatus('success');
      setStatusMessage(`Successfully extracted ${response.data.components.length} components.`);
      
      const diagram = toWokwiDiagram(response.data);
      setWokwiDiagram(diagram);

      const code = response.data.code || '// No code found';
      setExtractedCode(code);
      setArduinoCode(code);

      console.log('Wokwi Diagram:', diagram);
    } else {
      setStatus('error');
      setStatusMessage(response.message || 'Extraction failed.');
    }
  };

  return (
    <div id="labwise-app" style={{ display: 'flex', width: '100vw', height: '100vh', flexDirection: 'column', background: '#0f0f1a', color: '#fff' }}>
      
      <header style={{ padding: '16px 24px', borderBottom: '1px solid rgba(255,255,255,0.1)', display: 'flex', alignItems: 'center' }}>
        <div>
          <h1 style={{ margin: 0, fontSize: '1.25rem', fontWeight: 600 }}>LabWise</h1>
          <p style={{ margin: 0, fontSize: '0.75rem', opacity: 0.6 }}>AI Pre-Lab Environment</p>
        </div>
      </header>

      <main style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
        
        <section style={{ width: '400px', borderRight: '1px solid rgba(255,255,255,0.1)', display: 'flex', flexDirection: 'column' }}>
          <div style={{ padding: '16px', borderBottom: '1px solid rgba(255,255,255,0.1)' }}>
            <h2 style={{ fontSize: '1rem', margin: '0 0 8px 0' }}>1. Paste Lab Instructions</h2>
            <textarea 
              placeholder="Paste your lab manual text here..."
              value={labInstructions}
              onChange={(e) => setLabInstructions(e.target.value)}
              style={{ width: '100%', height: '150px', background: 'rgba(0,0,0,0.2)', border: '1px solid rgba(255,255,255,0.2)', color: '#fff', padding: '12px', borderRadius: '4px', resize: 'none' }}
            />
            <button 
              onClick={handleExtract}
              disabled={status === 'loading'}
              style={{ 
                marginTop: '8px', 
                width: '100%', 
                padding: '8px', 
                background: status === 'loading' ? '#444' : '#6366f1', 
                color: '#fff', 
                border: 'none', 
                borderRadius: '4px', 
                cursor: status === 'loading' ? 'not-allowed' : 'pointer' 
              }}
            >
              {status === 'loading' ? 'Extracting...' : 'Extract Circuit & Code'}
            </button>
          </div>
          <div style={{ padding: '16px', flex: 1, overflowY: 'auto' }}>
            <h2 style={{ fontSize: '1rem', margin: '0 0 8px 0' }}>AI Extraction Status</h2>
            <div style={{ 
              fontSize: '0.85rem', 
              color: status === 'error' ? '#ef4444' : (status === 'success' ? '#10b981' : '#fff'),
              opacity: status === 'idle' ? 0.5 : 1
            }}>
              {statusMessage || 'Waiting for input...'}
            </div>

            {extractedCode && status === 'success' && (
              <div style={{ marginTop: '20px' }}>
                <h3 style={{ fontSize: '0.9rem', color: '#6366f1' }}>Extracted Code:</h3>
                <pre style={{ 
                  background: 'rgba(0,0,0,0.4)', 
                  padding: '10px', 
                  borderRadius: '4px', 
                  fontSize: '0.75rem', 
                  overflow: 'auto',
                  maxHeight: '300px'
                }}>
                  {extractedCode}
                </pre>
              </div>
            )}
          </div>
        </section>

        <section style={{ flex: 1, display: 'flex', flexDirection: 'column', padding: '24px' }}>
          {wokwiDiagram ? (
            <WokwiEmbed diagram={wokwiDiagram} code={extractedCode} />
          ) : (
            <div style={{ flex: 1, background: 'rgba(0,0,0,0.3)', borderRadius: '8px', border: '1px dashed rgba(255,255,255,0.2)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
              <div style={{ textAlign: 'center' }}>
                <h2 style={{ fontSize: '1.5rem', opacity: 0.8, marginBottom: '8px' }}>Wokwi Simulation Environment</h2>
                <p style={{ opacity: 0.5 }}>Extract a circuit to launch the emulator.</p>
              </div>
            </div>
          )}
        </section>

      </main>
    </div>
  );
}

export default App;
