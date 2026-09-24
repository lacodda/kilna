import { fileURLToPath } from 'node:url'
import { defineConfig } from 'vitest/config'

// The frontend had no test runner at all until v0.17; this is the smallest
// thing that runs the pure logic under `src/lib`. A component test here
// renders to markup on the server - no DOM environment yet - which is enough
// when the question is the shape of what the browser receives. Mounted
// component tests arrive with the wider safety net in v0.77.
export default defineConfig({
  resolve: {
    alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
  },
  test: {
    include: ['src/**/*.test.ts', 'src/**/*.test.tsx'],
  },
})
