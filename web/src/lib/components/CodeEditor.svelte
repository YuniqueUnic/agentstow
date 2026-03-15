<script lang="ts">
  import { onDestroy, onMount } from 'svelte';

  import type { CompletionSource } from '@codemirror/autocomplete';
  import type { Compartment as CompartmentType, Extension } from '@codemirror/state';
  import type { Decoration, DecorationSet, ViewUpdate } from '@codemirror/view';
  import type { EditorView as EditorViewType } from 'codemirror';

  import type { EditorDocumentLanguage } from '$lib/types';

  type ResolvedEditorLanguage =
    | Exclude<EditorDocumentLanguage, 'auto'>
    | 'jinja-toml'
    | 'jinja-json'
    | 'jinja-shell';

  type OverlayLanguage = 'toml' | 'json' | 'shell';

  type Props = {
    value: string;
    readonly?: boolean;
    onChange?: (next: string) => void;
    testId?: string;
    documentLanguage?: EditorDocumentLanguage;
    documentPath?: string | null;
  };

  type LoadedCodeMirror = {
    state: typeof import('@codemirror/state');
    view: typeof import('@codemirror/view');
    cm: typeof import('codemirror');
    autocomplete: typeof import('@codemirror/autocomplete');
    commands: typeof import('@codemirror/commands');
    language: typeof import('@codemirror/language');
    langJinja: typeof import('@codemirror/lang-jinja');
    langJavascript: typeof import('@codemirror/lang-javascript');
    langHtml: typeof import('@codemirror/lang-html');
    langCss: typeof import('@codemirror/lang-css');
  };

  type CompletionEntry = {
    label: string;
    detail?: string;
    type?: string;
    apply?: string;
  };

  type DecorationPattern = {
    regexp: RegExp;
    className: string;
    group?: number;
  };

  const TOML_PATTERNS: DecorationPattern[] = [
    { regexp: /(^|\n)(\s*\[\[?[^[\]\n]+]]?\s*)/gd, className: 'cm-doc-token-table', group: 2 },
    { regexp: /(^|\n)(\s*)([A-Za-z0-9_.-]+)(\s*)(?==)/gd, className: 'cm-doc-token-key', group: 3 },
    { regexp: /"(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'/gd, className: 'cm-doc-token-string' },
    { regexp: /\b(true|false)\b/gd, className: 'cm-doc-token-bool' },
    {
      regexp: /\b[-+]?(?:\d[\d_]*)(?:\.\d[\d_]*)?(?:[eE][-+]?\d+)?\b/gd,
      className: 'cm-doc-token-number'
    },
    { regexp: /#[^\n]*/gd, className: 'cm-doc-token-comment' }
  ];

  const JSON_PATTERNS: DecorationPattern[] = [
    {
      regexp: /("(?:[^"\\]|\\.)*")(\s*:)/gd,
      className: 'cm-doc-token-key',
      group: 1
    },
    { regexp: /"(?:[^"\\]|\\.)*"/gd, className: 'cm-doc-token-string' },
    { regexp: /\b(true|false)\b/gd, className: 'cm-doc-token-bool' },
    { regexp: /\bnull\b/gd, className: 'cm-doc-token-null' },
    {
      regexp: /\b-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?\b/gd,
      className: 'cm-doc-token-number'
    }
  ];

  const SHELL_PATTERNS: DecorationPattern[] = [
    { regexp: /^#![^\n]*/gd, className: 'cm-doc-token-meta' },
    { regexp: /#[^\n]*/gd, className: 'cm-doc-token-comment' },
    { regexp: /\$\{?[A-Za-z_][A-Za-z0-9_]*}?/gd, className: 'cm-doc-token-keyword' },
    { regexp: /"(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'/gd, className: 'cm-doc-token-string' }
  ];

  const TOKEN_THEME = {
    '.cm-doc-token-table': {
      color: 'var(--primary)',
      fontWeight: '700'
    },
    '.cm-doc-token-key': {
      color: 'color-mix(in oklch, var(--ink) 70%, var(--primary))',
      fontWeight: '600'
    },
    '.cm-doc-token-string': {
      color: 'color-mix(in oklch, var(--primary) 56%, var(--ink))'
    },
    '.cm-doc-token-number, .cm-doc-token-bool, .cm-doc-token-null': {
      color: 'color-mix(in oklch, var(--ink) 60%, var(--primary))'
    },
    '.cm-doc-token-comment': {
      color: 'var(--ink-soft)',
      fontStyle: 'italic'
    },
    '.cm-doc-token-meta, .cm-doc-token-keyword': {
      color: 'color-mix(in oklch, var(--primary) 62%, var(--ink))'
    }
  } satisfies Parameters<typeof import('codemirror').EditorView.baseTheme>[0];

  const TOML_COMPLETIONS: CompletionEntry[] = [
    { label: 'kind = "file"', detail: 'artifact kind', type: 'keyword' },
    { label: 'source = "artifacts/example.txt.tera"', detail: 'source path', type: 'property' },
    { label: 'template = true', detail: 'render as template', type: 'keyword' },
    { label: 'validate_as = "toml"', detail: 'validation mode', type: 'keyword' },
    { label: '[profiles.base]', detail: 'profile section', type: 'class' },
    { label: '[artifacts.hello]', detail: 'artifact section', type: 'class' },
    { label: '[targets.out]', detail: 'target section', type: 'class' },
    { label: '[env_sets.default]', detail: 'env set section', type: 'class' },
    { label: '[scripts.sync]', detail: 'script section', type: 'class' },
    { label: '[mcp_servers.local]', detail: 'MCP server section', type: 'class' },
    { label: 'method = "copy"', detail: 'install method', type: 'enum' },
    { label: 'profile = "base"', detail: 'target profile', type: 'property' }
  ];

  const JSON_COMPLETIONS: CompletionEntry[] = [
    { label: '"key"', detail: 'object key', type: 'property' },
    { label: 'true', detail: 'boolean', type: 'constant' },
    { label: 'false', detail: 'boolean', type: 'constant' },
    { label: 'null', detail: 'null literal', type: 'constant' },
    { label: '{}', detail: 'object', type: 'class' },
    { label: '[]', detail: 'array', type: 'class' }
  ];

  const JINJA_COMPLETIONS: CompletionEntry[] = [
    { label: '{{ variable }}', detail: 'Tera placeholder', type: 'keyword' },
    { label: '{% if condition %}', detail: 'conditional block', type: 'keyword' },
    { label: '{% else %}', detail: 'conditional branch', type: 'keyword' },
    { label: '{% endif %}', detail: 'conditional end', type: 'keyword' },
    { label: '{% for item in items %}', detail: 'loop block', type: 'keyword' },
    { label: '{% endfor %}', detail: 'loop end', type: 'keyword' }
  ];

  const SHELL_COMPLETIONS: CompletionEntry[] = [
    { label: 'export KEY=value', detail: 'export variable', type: 'keyword' },
    { label: 'printf "%s\\n" "$VALUE"', detail: 'print value', type: 'function' },
    { label: '${OPENAI_API_KEY}', detail: 'env placeholder', type: 'variable' }
  ];

  const TEMPLATE_SUFFIXES = ['.tera', '.j2', '.jinja', '.jinja2'] as const;

  let {
    value,
    readonly = false,
    onChange,
    testId,
    documentLanguage = 'auto',
    documentPath = null
  }: Props = $props();

  let host: HTMLDivElement | null = null;
  let view: EditorViewType | null = null;
  let lastFromEditor = '';
  let activeTheme: 'light' | 'dark' = 'dark';
  let activeLanguage = $state<ResolvedEditorLanguage>('plaintext');

  let editable: CompartmentType | null = null;
  let themeConfig: CompartmentType | null = null;
  let languageConfig: CompartmentType | null = null;
  let completionConfig: CompartmentType | null = null;
  let attrsConfig: CompartmentType | null = null;
  let cmModule: typeof import('codemirror') | null = null;
  let codeMirrorDeps: LoadedCodeMirror | null = null;
  let loading = $state(true);
  let loadError = $state<string | null>(null);
  let alive = true;
  let themeObserver: MutationObserver | null = null;
  let mediaQuery: MediaQueryList | null = null;

  function normalizePath(path: string | null | undefined): string | null {
    if (!path) {
      return null;
    }

    const trimmed = path.trim();
    return trimmed ? trimmed.toLowerCase() : null;
  }

  function stripTemplateSuffix(normalized: string): string | null {
    for (const suffix of TEMPLATE_SUFFIXES) {
      if (normalized.endsWith(suffix)) {
        return normalized.slice(0, -suffix.length);
      }
    }

    return null;
  }

  function inferHostLanguageFromNormalizedPath(normalized: string): Exclude<
    ResolvedEditorLanguage,
    'jinja' | 'jinja-toml' | 'jinja-json' | 'jinja-shell'
  > | null {
    if (normalized.endsWith('.toml')) {
      return 'toml';
    }
    if (normalized.endsWith('.json') || normalized.endsWith('.jsonc')) {
      return 'json';
    }
    if (
      normalized.endsWith('.html') ||
      normalized.endsWith('.htm') ||
      normalized.endsWith('.xml') ||
      normalized.endsWith('.svg')
    ) {
      return 'html';
    }
    if (
      normalized.endsWith('.js') ||
      normalized.endsWith('.mjs') ||
      normalized.endsWith('.cjs') ||
      normalized.endsWith('.ts') ||
      normalized.endsWith('.tsx')
    ) {
      return 'javascript';
    }
    if (normalized.endsWith('.css') || normalized.endsWith('.scss') || normalized.endsWith('.less')) {
      return 'css';
    }
    if (
      normalized.endsWith('.sh') ||
      normalized.endsWith('.bash') ||
      normalized.endsWith('.zsh') ||
      normalized.endsWith('.fish') ||
      normalized.endsWith('.ps1') ||
      normalized.endsWith('.cmd') ||
      normalized.endsWith('.bat')
    ) {
      return 'shell';
    }

    return null;
  }

  function inferLanguageFromPath(path: string | null | undefined): ResolvedEditorLanguage | null {
    const normalized = normalizePath(path);
    if (!normalized) {
      return null;
    }

    const templateBase = stripTemplateSuffix(normalized);
    if (templateBase) {
      const hostLanguage = inferHostLanguageFromNormalizedPath(templateBase);
      if (hostLanguage === 'toml') {
        return 'jinja-toml';
      }
      if (hostLanguage === 'json') {
        return 'jinja-json';
      }
      if (hostLanguage === 'shell') {
        return 'jinja-shell';
      }
      return 'jinja';
    }

    return inferHostLanguageFromNormalizedPath(normalized);
  }

  function looksLikeJson(text: string): boolean {
    const trimmed = text.trim();
    if (!trimmed || !/^[{\[]/.test(trimmed)) {
      return false;
    }

    try {
      JSON.parse(trimmed);
      return true;
    } catch {
      return false;
    }
  }

  function looksLikeToml(text: string): boolean {
    const meaningfulLines = text
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter((line) => line.length > 0 && !line.startsWith('#'));

    if (meaningfulLines.length === 0) {
      return false;
    }

    if (meaningfulLines.some((line) => /^\[\[?[^[\]\n]+]]?$/.test(line))) {
      return true;
    }

    const assignmentCount = meaningfulLines.filter((line) => /^[A-Za-z0-9_.-]+\s*=/.test(line)).length;
    return assignmentCount >= Math.max(1, Math.ceil(meaningfulLines.length / 3));
  }

  function looksLikeShell(text: string): boolean {
    const trimmed = text.trim();
    if (!trimmed) {
      return false;
    }

    return (
      trimmed.startsWith('#!') ||
      /\b(export|unset|printf|echo|if|then|fi)\b/.test(trimmed) ||
      /\$\{?[A-Za-z_][A-Za-z0-9_]*}?/.test(trimmed)
    );
  }

  function looksLikeHtml(text: string): boolean {
    const trimmed = text.trim();
    return /^<!?[A-Za-z][\s\S]*>$/m.test(trimmed) || /^<\w+[\s\S]*<\/\w+>$/.test(trimmed);
  }

  function resolveDocumentLanguage(): ResolvedEditorLanguage {
    if (documentLanguage !== 'auto') {
      return documentLanguage;
    }

    const fromPath = inferLanguageFromPath(documentPath);
    if (fromPath) {
      return fromPath;
    }

    const hasTemplateSyntax = /\{\{|\{%|\{#/.test(value);
    if (hasTemplateSyntax) {
      if (looksLikeJson(value)) {
        return 'jinja-json';
      }
      if (looksLikeToml(value)) {
        return 'jinja-toml';
      }
      if (looksLikeShell(value)) {
        return 'jinja-shell';
      }
      return 'jinja';
    }
    if (looksLikeJson(value)) {
      return 'json';
    }
    if (looksLikeToml(value)) {
      return 'toml';
    }
    if (looksLikeHtml(value)) {
      return 'html';
    }
    if (looksLikeShell(value)) {
      return 'shell';
    }

    return 'plaintext';
  }

  function isTypescriptLike(path: string | null | undefined): boolean {
    const normalized = normalizePath(path);
    return Boolean(normalized && (normalized.endsWith('.ts') || normalized.endsWith('.tsx')));
  }

  function mergeCompletionEntries(...groups: CompletionEntry[][]): CompletionEntry[] {
    const merged = new Map<string, CompletionEntry>();

    for (const group of groups) {
      for (const entry of group) {
        if (!merged.has(entry.label)) {
          merged.set(entry.label, entry);
        }
      }
    }

    return Array.from(merged.values());
  }

  function overlayPatternsFor(language: OverlayLanguage): DecorationPattern[] {
    switch (language) {
      case 'toml':
        return TOML_PATTERNS;
      case 'json':
        return JSON_PATTERNS;
      case 'shell':
        return SHELL_PATTERNS;
    }
  }

  function buildCompositeJinjaExtensions(
    deps: LoadedCodeMirror,
    overlayLanguage: OverlayLanguage
  ): Extension[] {
    return [deps.langJinja.jinja(), ...buildRegexHighlightExtension(deps, overlayPatternsFor(overlayLanguage))];
  }

  function buildTheme(
    cm: typeof import('codemirror'),
    theme: 'light' | 'dark'
  ): ReturnType<typeof cm.EditorView.theme> {
    return cm.EditorView.theme(
      {
        '&.cm-editor': {
          height: '100%',
          minHeight: '0',
          minWidth: '0',
          display: 'flex',
          flexDirection: 'column',
          background: 'transparent',
          color: 'var(--ink)',
          fontFamily: '"IBM Plex Mono", "SFMono-Regular", monospace',
          fontSize: '13px'
        },
        '.cm-scroller': {
          overflow: 'auto',
          minHeight: '0',
          minWidth: '0',
          flex: '1 1 auto'
        },
        '.cm-content': {
          minHeight: '100%',
          minWidth: '0',
          boxSizing: 'border-box',
          padding: '16px 18px 36px'
        },
        '.cm-line': {
          padding: '0 8px'
        },
        '.cm-gutters': {
          minHeight: '100%',
          background: 'transparent',
          border: 'none',
          color: 'color-mix(in oklch, var(--ink-muted) 88%, transparent)'
        },
        '.cm-activeLine': {
          background: 'color-mix(in oklch, var(--primary) 12%, transparent)'
        },
        '.cm-activeLineGutter': {
          background: 'transparent',
          color: 'var(--ink)'
        },
        '.cm-selectionBackground': {
          background: 'color-mix(in oklch, var(--primary) 32%, var(--canvas-deep))'
        },
        '&.cm-focused .cm-selectionBackground': {
          background: 'color-mix(in oklch, var(--primary) 36%, var(--canvas-deep))'
        },
        '&.cm-focused': {
          outline: '1px solid color-mix(in oklch, var(--primary) 42%, transparent)',
          borderRadius: '0',
          boxShadow: '0 0 0 3px color-mix(in oklch, var(--primary) 16%, transparent)'
        },
        '.cm-cursor': {
          borderLeftColor: 'color-mix(in oklch, var(--ink) 78%, transparent)'
        },
        '.cm-tooltip, .cm-panels': {
          background: 'var(--panel-elevated)',
          color: 'var(--ink)',
          border: '1px solid color-mix(in oklch, var(--line) 90%, transparent)'
        },
        '.cm-tooltip-autocomplete ul li[aria-selected]': {
          background: 'color-mix(in oklch, var(--primary) 16%, transparent)',
          color: 'var(--ink)'
        }
      },
      { dark: theme === 'dark' }
    );
  }

  function readResolvedTheme(): 'light' | 'dark' {
    if (typeof document === 'undefined') {
      return 'dark';
    }

    const attr = document.documentElement.dataset.theme;
    if (attr === 'light' || attr === 'dark') {
      return attr;
    }

    if (
      typeof window !== 'undefined' &&
      typeof window.matchMedia === 'function' &&
      window.matchMedia('(prefers-color-scheme: dark)').matches
    ) {
      return 'dark';
    }

    return 'light';
  }

  function buildRegexHighlightExtension(deps: LoadedCodeMirror, patterns: DecorationPattern[]): Extension[] {
    const { Decoration, EditorView, ViewPlugin } = deps.view;

    const buildDecorations = (text: string): DecorationSet => {
      const builder = new deps.state.RangeSetBuilder<Decoration>();

      for (const pattern of patterns) {
        pattern.regexp.lastIndex = 0;

        for (let match = pattern.regexp.exec(text); match; match = pattern.regexp.exec(text)) {
          const indices = (match as RegExpMatchArray & {
            indices?: Array<[number, number] | undefined>;
          }).indices;
          const range = indices?.[pattern.group ?? 0];

          if (range && range[0] < range[1]) {
            builder.add(range[0], range[1], Decoration.mark({ class: pattern.className }));
          }

          if (match[0].length === 0) {
            pattern.regexp.lastIndex += 1;
          }
        }
      }

      return builder.finish();
    };

    return [
      EditorView.baseTheme(TOKEN_THEME),
      ViewPlugin.fromClass(
        class {
          decorations: DecorationSet;

          constructor(initialView: { state: { doc: { toString(): string } } }) {
            this.decorations = buildDecorations(initialView.state.doc.toString());
          }

          update(update: { docChanged: boolean; viewportChanged: boolean; state: { doc: { toString(): string } } }) {
            if (!update.docChanged && !update.viewportChanged) {
              return;
            }

            this.decorations = buildDecorations(update.state.doc.toString());
          }
        },
        {
          decorations: (plugin) => plugin.decorations
        }
      )
    ];
  }

  function completionEntriesFor(language: ResolvedEditorLanguage): CompletionEntry[] {
    switch (language) {
      case 'jinja':
        return JINJA_COMPLETIONS;
      case 'jinja-toml':
        return mergeCompletionEntries(JINJA_COMPLETIONS, TOML_COMPLETIONS);
      case 'jinja-json':
        return mergeCompletionEntries(JINJA_COMPLETIONS, JSON_COMPLETIONS);
      case 'jinja-shell':
        return mergeCompletionEntries(JINJA_COMPLETIONS, SHELL_COMPLETIONS);
      case 'toml':
        return TOML_COMPLETIONS;
      case 'json':
        return JSON_COMPLETIONS;
      case 'shell':
        return SHELL_COMPLETIONS;
      default:
        return [];
    }
  }

  function buildLanguageExtensions(
    deps: LoadedCodeMirror,
    language: ResolvedEditorLanguage
  ): Extension[] {
    const extensions: Extension[] = [deps.language.indentUnit.of('  ')];

    switch (language) {
      case 'jinja':
        extensions.push(deps.langJinja.jinja());
        break;
      case 'jinja-toml':
        extensions.push(...buildCompositeJinjaExtensions(deps, 'toml'));
        break;
      case 'jinja-json':
        extensions.push(...buildCompositeJinjaExtensions(deps, 'json'));
        break;
      case 'jinja-shell':
        extensions.push(...buildCompositeJinjaExtensions(deps, 'shell'));
        break;
      case 'html':
        extensions.push(deps.langHtml.html());
        break;
      case 'javascript':
        extensions.push(
          deps.langJavascript.javascript({
            jsx: documentPath?.toLowerCase().endsWith('.tsx') ?? false,
            typescript: isTypescriptLike(documentPath)
          })
        );
        break;
      case 'css':
        extensions.push(deps.langCss.css());
        break;
      case 'toml':
        extensions.push(...buildRegexHighlightExtension(deps, TOML_PATTERNS));
        break;
      case 'json':
        extensions.push(...buildRegexHighlightExtension(deps, JSON_PATTERNS));
        break;
      case 'shell':
        extensions.push(...buildRegexHighlightExtension(deps, SHELL_PATTERNS));
        break;
      case 'plaintext':
        break;
    }

    return extensions;
  }

  function buildCompletionExtension(
    deps: LoadedCodeMirror,
    language: ResolvedEditorLanguage
  ): Extension {
    const entries = completionEntriesFor(language);
    const sources: CompletionSource[] = [deps.autocomplete.completeAnyWord];

    if (entries.length > 0) {
      sources.unshift(deps.autocomplete.completeFromList(entries));
    }

    return deps.autocomplete.autocompletion({
      override: sources,
      activateOnTyping: true,
      icons: false
    });
  }

  function buildContentAttributes(
    cm: typeof import('codemirror'),
    language: ResolvedEditorLanguage
  ): Extension {
    return cm.EditorView.contentAttributes.of({
      'data-language': language,
      spellcheck: 'false',
      autocapitalize: 'off',
      autocorrect: 'off',
      autocomplete: 'off'
    });
  }

  function syncTheme(): void {
    if (!view || !themeConfig || !cmModule) {
      return;
    }

    const nextTheme = readResolvedTheme();
    if (nextTheme === activeTheme) {
      return;
    }

    activeTheme = nextTheme;
    view.dispatch({
      effects: themeConfig.reconfigure(buildTheme(cmModule, nextTheme))
    });
  }

  async function attachEditor(): Promise<void> {
    if (!host) {
      return;
    }

    loading = true;
    loadError = null;

    try {
      const [state, viewModule, cm, autocomplete, commands, language, langJinja, langJavascript, langHtml, langCss] =
        await Promise.all([
          import('@codemirror/state'),
          import('@codemirror/view'),
          import('codemirror'),
          import('@codemirror/autocomplete'),
          import('@codemirror/commands'),
          import('@codemirror/language'),
          import('@codemirror/lang-jinja'),
          import('@codemirror/lang-javascript'),
          import('@codemirror/lang-html'),
          import('@codemirror/lang-css')
        ]);

      if (!alive || !host) {
        return;
      }

      const deps: LoadedCodeMirror = {
        state,
        view: viewModule,
        cm,
        autocomplete,
        commands,
        language,
        langJinja,
        langJavascript,
        langHtml,
        langCss
      };

      editable = new state.Compartment();
      themeConfig = new state.Compartment();
      languageConfig = new state.Compartment();
      completionConfig = new state.Compartment();
      attrsConfig = new state.Compartment();
      cmModule = cm;
      codeMirrorDeps = deps;
      activeTheme = readResolvedTheme();
      activeLanguage = resolveDocumentLanguage();

      view = new cm.EditorView({
        parent: host,
        doc: value,
        extensions: [
          cm.basicSetup,
          themeConfig.of(buildTheme(cm, activeTheme)),
          languageConfig.of(buildLanguageExtensions(deps, activeLanguage)),
          completionConfig.of(buildCompletionExtension(deps, activeLanguage)),
          attrsConfig.of(buildContentAttributes(cm, activeLanguage)),
          viewModule.keymap.of([commands.indentWithTab]),
          cm.EditorView.lineWrapping,
          editable.of(cm.EditorView.editable.of(!readonly)),
          cm.EditorView.updateListener.of((update: ViewUpdate) => {
            if (!update.docChanged) {
              return;
            }

            const next = update.state.doc.toString();
            lastFromEditor = next;
            onChange?.(next);
          })
        ]
      });
      lastFromEditor = value;
    } catch (error) {
      loadError = error instanceof Error ? error.message : '编辑器加载失败。';
    } finally {
      loading = false;
    }
  }

  function detachEditor(): void {
    view?.destroy();
    view = null;
  }

  onMount(() => {
    const onMediaChange = () => {
      syncTheme();
    };

    if (typeof document !== 'undefined' && typeof MutationObserver === 'function') {
      themeObserver = new MutationObserver(() => {
        syncTheme();
      });
      themeObserver.observe(document.documentElement, {
        attributes: true,
        attributeFilter: ['data-theme']
      });
    }

    if (typeof window !== 'undefined' && typeof window.matchMedia === 'function') {
      mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
      if (typeof mediaQuery.addEventListener === 'function') {
        mediaQuery.addEventListener('change', onMediaChange);
      } else {
        mediaQuery.addListener(onMediaChange);
      }
    }

    void attachEditor();

    return () => {
      if (!mediaQuery) {
        return;
      }
      if (typeof mediaQuery.removeEventListener === 'function') {
        mediaQuery.removeEventListener('change', onMediaChange);
      } else {
        mediaQuery.removeListener(onMediaChange);
      }
    };
  });

  onDestroy(() => {
    alive = false;
    themeObserver?.disconnect();
    themeObserver = null;
    detachEditor();
  });

  $effect(() => {
    if (!view) {
      return;
    }

    if (value === lastFromEditor) {
      return;
    }

    const current = view.state.doc.toString();
    if (value === current) {
      lastFromEditor = value;
      return;
    }

    view.dispatch({
      changes: {
        from: 0,
        to: view.state.doc.length,
        insert: value
      }
    });
    lastFromEditor = value;
  });

  $effect(() => {
    if (!view || !editable || !cmModule) {
      return;
    }

    view.dispatch({
      effects: editable.reconfigure(cmModule.EditorView.editable.of(!readonly))
    });
  });

  $effect(() => {
    if (!view || !cmModule || !codeMirrorDeps || !languageConfig || !completionConfig || !attrsConfig) {
      return;
    }

    const nextLanguage = resolveDocumentLanguage();
    if (nextLanguage === activeLanguage) {
      return;
    }

    activeLanguage = nextLanguage;
    view.dispatch({
      effects: [
        languageConfig.reconfigure(buildLanguageExtensions(codeMirrorDeps, nextLanguage)),
        completionConfig.reconfigure(buildCompletionExtension(codeMirrorDeps, nextLanguage)),
        attrsConfig.reconfigure(buildContentAttributes(cmModule, nextLanguage))
      ]
    });
  });
</script>

<div class="editor" data-language={activeLanguage} data-testid={testId}>
  <div class="editor__host" bind:this={host}></div>
  {#if loading || loadError}
    <div class={['editor__loading', loadError ? 'editor__loading--error' : ''].join(' ')}>
      {#if loadError}
        <div class="editor__loading-title">编辑器加载失败</div>
        <div class="editor__loading-detail mono">{loadError}</div>
      {:else}
        <span class="editor__spinner" aria-hidden="true"></span>
        <div class="editor__loading-title">加载编辑器…</div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .editor {
    height: 100%;
    min-height: 0;
    min-width: 0;
    position: relative;
    display: grid;
    overflow: hidden;
    border-radius: 0;
    background: transparent;
    border: 0;
    box-shadow: none;
  }

  .editor__host {
    height: 100%;
    min-height: 0;
    min-width: 0;
    overflow: hidden;
  }

  .editor :global(.cm-editor) {
    height: 100%;
    min-height: 0;
    min-width: 0;
    border-radius: 0;
  }

  .editor :global(.cm-scroller),
  .editor :global(.cm-content),
  .editor :global(.cm-gutters) {
    min-height: 0;
    min-width: 0;
  }

  .editor__loading {
    position: absolute;
    inset: 0;
    border-radius: 0;
    border: 1px dashed color-mix(in oklch, var(--line) 70%, transparent);
    background: color-mix(in oklch, var(--canvas-deep) 74%, transparent);
    display: grid;
    place-items: center;
    gap: 10px;
    padding: 16px;
    text-align: center;
    color: var(--ink-soft);
  }

  .editor__spinner {
    width: 18px;
    height: 18px;
    border-radius: 999px;
    border: 2px solid color-mix(in oklch, var(--line-strong) 42%, transparent);
    border-top-color: var(--primary);
    animation: editor-spin 0.9s linear infinite;
  }

  .editor__loading-title {
    font-size: 13px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }

  .editor__loading-detail {
    font-size: 12px;
    max-width: 60ch;
    overflow-wrap: anywhere;
  }

  .editor__loading--error {
    border-style: solid;
    border-color: color-mix(in oklch, var(--danger) 34%, transparent);
    background: color-mix(in oklch, var(--danger) 10%, transparent);
    color: color-mix(in oklch, var(--danger) 70%, var(--ink));
  }

  @keyframes editor-spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
