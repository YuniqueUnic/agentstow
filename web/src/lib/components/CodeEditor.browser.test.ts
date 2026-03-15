import { render } from 'vitest-browser-svelte';
import { expect, test } from 'vitest';

import CodeEditor from './CodeEditor.svelte';

function queryEditorToken(testId: string, className: string): Element | null {
  return document.querySelector(`[data-testid="${testId}"] .${className}`);
}

test('infers jinja-toml for template toml sources', async () => {
  const screen = await render(CodeEditor, {
    value: '[profile]\nname = "{{ name }}"\n',
    documentPath: '/tmp/agentstow/profile.toml.tera',
    testId: 'code-editor'
  });

  await expect.element(screen.getByTestId('code-editor')).toHaveAttribute('data-language', 'jinja-toml');
  expect(queryEditorToken('code-editor', 'cm-doc-token-table')).not.toBeNull();
  expect(queryEditorToken('code-editor', 'cm-doc-token-key')).not.toBeNull();
});

test('infers jinja-json for template json sources', async () => {
  const screen = await render(CodeEditor, {
    value: '{\n  "name": "{{ name }}"\n}\n',
    documentPath: '/tmp/agentstow/config.json.tera',
    testId: 'code-editor'
  });

  await expect.element(screen.getByTestId('code-editor')).toHaveAttribute('data-language', 'jinja-json');
  expect(queryEditorToken('code-editor', 'cm-doc-token-key')).not.toBeNull();
});

test('uses structured JSON highlighting for plain json documents', async () => {
  const screen = await render(CodeEditor, {
    value: '{\n  "name": "AgentStow",\n  "enabled": true\n}\n',
    documentLanguage: 'json',
    testId: 'json-editor'
  });

  await expect.element(screen.getByTestId('json-editor')).toHaveAttribute('data-language', 'json');
  expect(queryEditorToken('json-editor', 'cm-doc-token-key')).not.toBeNull();
  expect(queryEditorToken('json-editor', 'cm-doc-token-string')).not.toBeNull();
});
