import fs from 'node:fs';
import path from 'node:path';
import { defineConfig } from 'vite';
// import react from '@vitejs/plugin-react';

const base = path.resolve('..');
const cargoManifest = path.resolve(base, '../mylife-home-studio/Cargo.toml');

// The version shown in the status bar is the one of the Rust crate the web app
// is served by, so a release is bumped in one place only. Read as text rather
// than through cargo, because the frontend build stage has no cargo.
function crateVersion() {
  const content = fs.readFileSync(cargoManifest, 'utf8');
  const section = content.split(/^\[/m).find((part) => part.startsWith('package]'));
  const found = section && /^\s*version\s*=\s*"([^"]+)"/m.exec(section);

  if (!found) {
    throw new Error(`no package version in ${cargoManifest}`);
  }

  return found[1];
}

export default defineConfig({
  root: path.resolve(base, 'src'),
  publicDir: path.resolve(base, 'static'),
  server: {
    host: true,
    port: 8102,
    allowedHosts: true,
    strictPort: true,
    proxy: {
      '/websocket': {
        target: 'ws://localhost:8002',
        ws: true,
      },
    },
  },
  define: {
    __APP_VERSION__: JSON.stringify(crateVersion()),
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
