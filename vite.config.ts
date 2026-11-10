import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { fileURLToPath } from 'node:url'
import pkg from './package.json' with { type: 'json' }

// Tauri serves the frontend from a fixed port and expects a static build in dist/.
export default defineConfig({
  plugins: [
    react(),
    tailwindcss(),
    // The splash in index.html prints the version before the bundle - and
    // `define` below - has loaded, so the number is written into the page
    // itself at build time.
    {
      name: 'kilna:splash-version',
      transformIndexHtml: (html) => html.replace('__APP_VERSION__', pkg.version),
    },
  ],
  define: {
    __APP_VERSION__: JSON.stringify(pkg.version),
  },
  resolve: {
    alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
  },
  build: {
    // A font is never written into the bundle as `data:`: the window's CSP
    // (`default-src 'self'`, no `font-src`) refuses one without a word, and a
    // style sample in Pixelify Sans bold fell back to another face (wish
    // 6850). Everything else follows Vite's own limit.
    assetsInlineLimit: (file) => (/\.(woff2?|ttf|otf)$/i.test(file) ? false : undefined),
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ['**/src-tauri/**'] },
  },
})
