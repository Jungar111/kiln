import { defineConfig } from 'vite';
import { sveltekit } from '@sveltejs/kit/vite';
import process from 'node:process';

// `tauri dev --host` plumbs the LAN address through this env var so the
// webview reaches the dev server when running on a different device.
const host = process.env.TAURI_DEV_HOST;

const hmr = host ? { protocol: 'ws', host, port: 1421 } : true;

// https://vite.dev/config/
export default defineConfig({
  plugins: [sveltekit()],

  // Surface Rust errors instead of clearing the terminal.
  clearScreen: false,
  server: {
    // Tauri expects a fixed port and fails if it is unavailable.
    port: 1420,
    strictPort: true,
    host: host ?? false,
    hmr,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
});
