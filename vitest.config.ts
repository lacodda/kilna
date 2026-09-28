import { fileURLToPath } from 'node:url'
import { defineConfig } from 'vitest/config'
import pkg from './package.json' with { type: 'json' }

// Dates on screen are drawn in local time, and the tests should not read
// "07:00" on one machine and "10:00" on another. Set before any worker starts.
process.env.TZ = 'UTC'

// Two runs over one tree. The pure logic under `src/lib` needs no document and
// runs in Node, which is fast; a component or a hook needs one, and gets jsdom
// with the app's providers and a mocked backend (`src/test/`). The file says
// which: a `.test.tsx` renders, a `.test.ts` does not.
export default defineConfig({
  resolve: {
    alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
  },
  define: {
    __APP_VERSION__: JSON.stringify(pkg.version),
  },
  test: {
    projects: [
      {
        extends: true,
        test: {
          name: 'logic',
          include: ['src/**/*.test.ts'],
          environment: 'node',
        },
      },
      {
        extends: true,
        test: {
          name: 'dom',
          include: ['src/**/*.test.tsx'],
          environment: 'jsdom',
          setupFiles: ['src/test/setup.ts'],
          // A whole screen settles in well under a second; the default five
          // leaves room for a slow machine without hiding a hang.
          testTimeout: 15_000,
        },
      },
    ],
  },
})
