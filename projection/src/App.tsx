/**
 * LabWise — Main Application
 *
 * Pre-Lab Assistant: AI extraction -> Netlist -> Wokwi execution.
 */

import { useState } from 'react';
import './App.css';

function App() {
  return (
    <div id="labwise-app" style={{ display: 'flex', width: '100vw', height: '100vh', flexDirection: 'column', background: '#0f0f1a', color: '#fff' }}>
      
      {/* Header */}
      <header style={{ padding: '16px 24px', borderBottom: '1px solid rgba(255,255,255,0.1)', display: 'flex', alignItems: 'center' }}>
        <div>
          <h1 style={{ margin: 0, fontSize: '1.25rem', fontWeight: 600 }}>LabWise</h1>
          <p style={{ margin: 0, fontSize: '0.75rem', opacity: 0.6 }}>AI Pre-Lab Environment</p>
        </div>
      </header>

      {/* Main Content Workspace */}
      <main style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
        
        {/* Left Sidebar: Lab Manual Input & AI Chat */}
        <section style={{ width: '400px', borderRight: '1px solid rgba(255,255,255,0.1)', display: 'flex', flexDirection: 'column' }}>
          <div style={{ padding: '16px', borderBottom: '1px solid rgba(255,255,255,0.1)' }}>
            <h2 style={{ fontSize: '1rem', margin: '0 0 8px 0' }}>1. Paste Lab Instructions</h2>
            <textarea 
              placeholder="Paste your lab manual text here..."
              style={{ width: '100%', height: '150px', background: 'rgba(0,0,0,0.2)', border: '1px solid rgba(255,255,255,0.2)', color: '#fff', padding: '12px', borderRadius: '4px', resize: 'none' }}
            />
            <button style={{ marginTop: '8px', width: '100%', padding: '8px', background: '#6366f1', color: '#fff', border: 'none', borderRadius: '4px', cursor: 'pointer' }}>
              Extract Circuit & Code
            </button>
          </div>
          <div style={{ padding: '16px', flex: 1, overflowY: 'auto' }}>
            <h2 style={{ fontSize: '1rem', margin: '0 0 8px 0' }}>AI Extraction Status</h2>
            <div style={{ opacity: 0.5, fontSize: '0.85rem' }}>Waiting for input...</div>
          </div>
        </section>

        {/* Center/Right: Circuit View & Wokwi Simulation */}
        <section style={{ flex: 1, display: 'flex', flexDirection: 'column', padding: '24px' }}>
          <div style={{ flex: 1, background: 'rgba(0,0,0,0.3)', borderRadius: '8px', border: '1px dashed rgba(255,255,255,0.2)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <div style={{ textAlign: 'center' }}>
              <h2 style={{ fontSize: '1.5rem', opacity: 0.8, marginBottom: '8px' }}>Wokwi Simulation Environment</h2>
              <p style={{ opacity: 0.5 }}>Extract a circuit to launch the emulator.</p>
            </div>
          </div>
        </section>

      </main>
    </div>
  );
}

export default App;
