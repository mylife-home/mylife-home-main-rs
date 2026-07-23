import path from 'node:path';
import { defineConfig } from 'vite';
// import react from '@vitejs/plugin-react';

const base = path.resolve('..');

export default defineConfig({
  root: path.resolve(base, 'src'),
  publicDir: path.resolve(base, 'static'),
  server: {
    host: true,
    port: 8102,
    allowedHosts: true,
    strictPort: true,
    proxy: {
      '/socket.io': {
        target: 'ws://localhost:8002',
        ws: true,
      },
    },
  },
  build: {
    outDir: path.resolve(base, 'dist'),
    emptyOutDir: true,
    // rollupOptions: {
    //   // Seems side effects are dropped else
    //   // on Money changes size from 3.4Mb to 3.7Mb
    //   treeshake: false 
    // }
  },
  // plugins: [ react() ],
});
