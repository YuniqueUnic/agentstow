/// <reference types="vite/client" />

import type { HTMLAttributes } from 'svelte/elements';

declare module 'svelte/elements' {
  interface MaterialBaseAttributes extends HTMLAttributes<HTMLElement> {
    disabled?: boolean;
    indeterminate?: boolean;
    label?: string;
    placeholder?: string;
    rows?: number | string;
    selected?: boolean;
    type?: string;
    value?: string;
    'aria-label'?: string;
    'error-text'?: string;
    'supporting-text'?: string;
  }

  export interface SvelteHTMLElements {
    'md-chip-set': HTMLAttributes<HTMLElement>;
    'md-circular-progress': MaterialBaseAttributes;
    'md-divider': HTMLAttributes<HTMLElement>;
    'md-filled-tonal-button': MaterialBaseAttributes;
    'md-filter-chip': MaterialBaseAttributes;
    'md-outlined-button': MaterialBaseAttributes;
    'md-outlined-text-field': MaterialBaseAttributes;
    'md-text-button': MaterialBaseAttributes;
  }
}

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
