import { fileURLToPath } from 'node:url'
import { defineConfig } from 'vitest/config'

// The frontend had no test runner at all until v0.17; this is the smallest
// thing that runs the pure logic under `src/lib`. Component tests need a DOM
// environment and belong with the wider testing pass in the 0.41 block.
export default defineConfig({
  resolve: {
    alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
  },
  test: {
    include: ['src/**/*.test.ts'],
  },
})
