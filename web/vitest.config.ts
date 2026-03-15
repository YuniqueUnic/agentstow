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
    include: [
      '@codemirror/autocomplete',
      '@codemirror/commands',
      '@codemirror/lang-css',
      '@codemirror/lang-html',
      '@codemirror/lang-javascript',
      '@codemirror/lang-jinja',
      '@codemirror/lang-json',
      '@codemirror/language',
      '@codemirror/legacy-modes/mode/toml',
      '@codemirror/state',
      '@codemirror/view',
      'codemirror'
    ]
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
