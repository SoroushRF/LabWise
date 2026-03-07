import React, { useState, useRef } from 'react';
import { extractCircuit } from './synapse';
import { useCircuitStore } from './store/circuitStore';
import { toWokwiDiagram, type WokwiDiagram } from './wokwi/Mapper';
import { CircuitBoard } from './wokwi/CircuitBoard';
import { extractTextFromPDF } from './utils/pdfExtractor';
import './App.css';

function App() {
  const [labInstructions, setLabInstructions] = useState('');
  const [status, setStatus] = useState<'idle' | 'loading' | 'success' | 'error'>('idle');
  const [statusMessage, setStatusMessage] = useState('');
  const [wokwiDiagram, setWokwiDiagram] = useState<WokwiDiagram | null>(null);
  const [extractedCode, setExtractedCode] = useState<string>('');
  const [isProcessingPdf, setIsProcessingPdf] = useState(false);
  const [rawAiResponse, setRawAiResponse] = useState<string>('');
  const [showDebug, setShowDebug] = useState(false);
  
  const fileInputRef = useRef<HTMLInputElement>(null);
  const setArduinoCode = useCircuitStore((state) => state.setArduinoCode);

  const handlePdfUpload = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    setIsProcessingPdf(true);
    setStatusMessage('Extracting text from PDF...');
    
    try {
      const text = await extractTextFromPDF(file);
      setLabInstructions(text);
      setStatus('idle');
      setStatusMessage('Text extracted from PDF. You can now tweak it or extract the circuit.');
    } catch (error) {
      console.error('PDF Extraction Error:', error);
      setStatus('error');
      setStatusMessage('Failed to read PDF. Try copy-pasting the text instead.');
    } finally {
      setIsProcessingPdf(false);
      // Reset input so the same file can be uploaded again
      if (fileInputRef.current) fileInputRef.current.value = '';
    }
  };

  const handleExtract = async () => {
    if (!labInstructions.trim()) return;

    setStatus('loading');
    setStatusMessage('Consulting Gemini AI...');
    setWokwiDiagram(null);

    const response = await extractCircuit(labInstructions);

    if (response.status === 'success') {
      setStatus('success');
      setStatusMessage(`Successfully extracted ${response.data.components.length} components.`);
      setRawAiResponse('');
      setShowDebug(false);
      
      const diagram = toWokwiDiagram(response.data);
      setWokwiDiagram(diagram);

      const code = response.data.code || '// No code found';
      setExtractedCode(code);
      setArduinoCode(code);

      console.log('Wokwi Diagram:', diagram);
    } else {
      setStatus('error');
      setRawAiResponse(response.rawOutput || '');
      
      // Better handling for 429 Rate Limits
      if (response.message.includes('429')) {
        setStatusMessage('AI is taking a breather (Rate Limited). Please wait 20-30 seconds and click Extract again.');
      } else {
        setStatusMessage(response.message || 'Extraction failed.');
      }
    }
  };

  return (
    <div id="labwise-app" style={{ display: 'flex', width: '100vw', height: '100vh', flexDirection: 'column', background: '#0f0f1a', color: '#fff' }}>
      
      <header style={{ padding: '16px 24px', borderBottom: '1px solid rgba(255,255,255,0.1)', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div>
          <h1 style={{ margin: 0, fontSize: '1.25rem', fontWeight: 600 }}>LabWise</h1>
          <p style={{ margin: 0, fontSize: '0.75rem', opacity: 0.6 }}>AI Pre-Lab Environment</p>
        </div>
        <div>
          <input 
            type="file" 
            accept="application/pdf" 
            ref={fileInputRef} 
            onChange={handlePdfUpload} 
            style={{ display: 'none' }} 
          />
          <button 
            onClick={() => fileInputRef.current?.click()}
            disabled={isProcessingPdf || status === 'loading'}
            style={{
              padding: '8px 16px',
              background: 'rgba(255,255,255,0.1)',
              border: '1px solid rgba(255,255,255,0.2)',
              color: '#fff',
              borderRadius: '6px',
              cursor: 'pointer',
              fontSize: '0.85rem',
              display: 'flex',
              alignItems: 'center',
              gap: '8px'
            }}
          >
            {isProcessingPdf ? 'Reading PDF...' : '📄 Upload PDF Manual'}
          </button>
        </div>
      </header>

      <main style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
        
        <section style={{ width: '400px', borderRight: '1px solid rgba(255,255,255,0.1)', display: 'flex', flexDirection: 'column' }}>
          <div style={{ padding: '16px', borderBottom: '1px solid rgba(255,255,255,0.1)' }}>
            <h2 style={{ fontSize: '1rem', margin: '0 0 8px 0' }}>1. Lab Instructions</h2>
            <textarea 
              placeholder="Paste manual text OR upload a PDF above..."
              value={labInstructions}
              onChange={(e) => setLabInstructions(e.target.value)}
              style={{ width: '100%', height: '200px', background: 'rgba(0,0,0,0.2)', border: '1px solid rgba(255,255,255,0.2)', color: '#fff', padding: '12px', borderRadius: '4px', resize: 'none' }}
            />
            <button 
              onClick={handleExtract}
              disabled={status === 'loading' || isProcessingPdf || !labInstructions.trim()}
              style={{ 
                marginTop: '8px', 
                width: '100%', 
                padding: '10px', 
                background: (status === 'loading' || isProcessingPdf || !labInstructions.trim()) ? '#444' : '#6366f1', 
                color: '#fff', 
                border: 'none', 
                borderRadius: '4px', 
                cursor: (status === 'loading' || isProcessingPdf || !labInstructions.trim()) ? 'not-allowed' : 'pointer',
                fontWeight: 600
              }}
            >
              {status === 'loading' ? 'Analyzing with Gemini...' : 'Extract Circuit & Code'}
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

            {status === 'error' && rawAiResponse && (
              <div style={{ marginTop: '12px' }}>
                <button 
                  onClick={() => setShowDebug(!showDebug)}
                  style={{ background: 'transparent', border: '1px solid #444', color: '#888', fontSize: '0.7rem', padding: '4px 8px', borderRadius: '4px', cursor: 'pointer' }}
                >
                  {showDebug ? 'Hide Raw AI Output' : 'Show Raw AI Output'}
                </button>
                {showDebug && (
                  <pre style={{ 
                    marginTop: '8px', 
                    background: 'rgba(255,0,0,0.05)', 
                    padding: '8px', 
                    borderRadius: '4px', 
                    fontSize: '0.65rem', 
                    color: '#fca5a5', 
                    overflow: 'auto',
                    border: '1px solid rgba(255,0,0,0.2)',
                    whiteSpace: 'pre-wrap'
                  }}>
                    {rawAiResponse}
                  </pre>
                )}
              </div>
            )}

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
            <CircuitBoard diagram={wokwiDiagram} />
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
