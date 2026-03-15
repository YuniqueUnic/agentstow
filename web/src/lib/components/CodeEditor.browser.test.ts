import { render } from 'vitest-browser-svelte';
import { expect, test } from 'vitest';

import CodeEditor from './CodeEditor.svelte';

test('infers jinja-toml for template toml sources', async () => {
  const screen = await render(CodeEditor, {
    value: '[profile]\nname = "{{ name }}"\n',
    documentPath: '/tmp/agentstow/profile.toml.tera',
    testId: 'code-editor'
  });

  await expect.element(screen.getByTestId('code-editor')).toHaveAttribute('data-language', 'jinja-toml');
});

test('infers jinja-json for template json sources', async () => {
  const screen = await render(CodeEditor, {
    value: '{\n  "name": "{{ name }}"\n}\n',
    documentPath: '/tmp/agentstow/config.json.tera',
    testId: 'code-editor'
  });

  await expect.element(screen.getByTestId('code-editor')).toHaveAttribute('data-language', 'jinja-json');
});
