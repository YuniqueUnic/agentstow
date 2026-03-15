import { render } from 'vitest-browser-svelte';
import { expect, test } from 'vitest';

import DiffViewer from './DiffViewer.svelte';

test('renders a structured diff hunk instead of raw git diff text', async () => {
  const screen = await render(DiffViewer, {
    original: 'Hello {{ name }}!\n',
    modified: 'Hello {{ name }} from Git!\n',
    fromLabel: 'saved',
    toLabel: 'worktree'
  });

  const diff = screen.getByLabelText('Structured diff preview');

  await expect.element(diff).toBeVisible();
  await expect.element(screen.getByText('@@ -1,1 +1,1 @@')).toBeVisible();
  await expect.element(screen.getByText('Hello {{ name }}!')).toBeVisible();
  await expect.element(screen.getByText('Hello {{ name }} from Git!')).toBeVisible();
});
