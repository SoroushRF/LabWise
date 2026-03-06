import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  optimizeDeps: {
    // Don't pre-bundle the WASM package — it needs to load the .wasm file at runtime
    exclude: ['labwise_bridge'],
  },
  server: {
    fs: {
      // Allow Vite dev server to serve files from the wasm-pkg directory
      allow: ['..'],
    },
  },
})
