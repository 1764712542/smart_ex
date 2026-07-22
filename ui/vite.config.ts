import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import { fileURLToPath } from 'node:url'

// https://vite.dev/config/
export default defineConfig({
  plugins: [svelte()],
  // Tauri expects a fixed port, fail if it's not available
  clearScreen: false,
  resolve: {
    alias: {
      $lib: fileURLToPath(new URL('./src/lib', import.meta.url)),
    },
  },
  server: {
    port: 5173,
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_ENV_*'],
  build: {
    // Tauri supports esbuild for smaller binaries
    target:
      process.env.TAURI_ENV_PLATFORM === 'windows'
        ? 'chrome105'
        : 'safari13',
    // produce sourcemaps for debug builds
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
})
