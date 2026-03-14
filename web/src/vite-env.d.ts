/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_AGENTSTOW_API_BASE?: string;
  readonly VITE_AGENTSTOW_API_ORIGIN?: string;
  readonly VITE_PORT?: string;
  readonly VITE_PREVIEW_PORT?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

export {};
