import { fileURLToPath, URL } from 'node:url';

import { svelte } from '@sveltejs/vite-plugin-svelte';
import { playwright } from '@vitest/browser-playwright';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [svelte()],
  resolve: {
    alias: {
      $lib: fileURLToPath(new URL('./src/lib', import.meta.url)),
      $src: fileURLToPath(new URL('./src', import.meta.url))
    }
  },
  optimizeDeps: {
    include: ['@codemirror/lang-jinja', '@codemirror/state', 'codemirror']
  },
  test: {
    include: ['src/**/*.browser.test.ts'],
    setupFiles: ['./tests/browser/setup.ts'],
    browser: {
      enabled: true,
      headless: true,
      provider: playwright(),
      viewport: {
        width: 1440,
        height: 960
      },
      instances: [{ browser: 'chromium' }]
    }
  }
});
