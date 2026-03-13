import { fileURLToPath, URL } from 'node:url';

import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig, loadEnv } from 'vite';

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '');
  const apiOrigin = env.VITE_AGENTSTOW_API_ORIGIN || 'http://127.0.0.1:8787';
  const devPort = Number(env.VITE_PORT || 5173);
  const previewPort = Number(env.VITE_PREVIEW_PORT || 4173);

  return {
    plugins: [svelte()],
    resolve: {
      alias: {
        $lib: fileURLToPath(new URL('./src/lib', import.meta.url)),
        $src: fileURLToPath(new URL('./src', import.meta.url))
      }
    },
    server: {
      host: '0.0.0.0',
      port: devPort,
      proxy: {
        '/api': {
          target: apiOrigin,
          changeOrigin: true
        }
      }
    },
    preview: {
      host: '0.0.0.0',
      port: previewPort
    },
    build: {
      target: 'es2022',
      sourcemap: true
    }
  };
});
