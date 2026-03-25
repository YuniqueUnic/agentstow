import { spawn } from 'node:child_process';
import { mkdir, mkdtemp, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';

import { expect, type Page } from '@playwright/test';

export const PLAYWRIGHT_WORKSPACE_ROOT = path.join(os.tmpdir(), 'agentstow-playwright-workspace');
export const PLAYWRIGHT_BOOTSTRAP_ROOT = path.join(
  os.tmpdir(),
  'agentstow-playwright-generated-workspace'
);

async function runCommand(command: string, args: string[], cwd: string): Promise<void> {
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd,
      stdio: ['ignore', 'pipe', 'pipe']
    });

    let stderr = '';
    child.stderr.on('data', (chunk) => {
      stderr += chunk.toString();
    });

    child.on('error', reject);
    child.on('exit', (code) => {
      if (code === 0) {
        resolve(undefined);
        return;
      }

      reject(new Error(`${command} ${args.join(' ')} failed (${code}): ${stderr}`));
    });
  });
}

export async function createWorkbenchWorkspace(prefix = 'agentstow-e2e-workspace-'): Promise<string> {
  const workspaceRoot = await mkdtemp(path.join(os.tmpdir(), prefix));
  await mkdir(path.join(workspaceRoot, 'artifacts'), { recursive: true });

  await writeFile(
    path.join(workspaceRoot, 'agentstow.toml'),
    `[profiles.base]
vars = { name = "AgentStow" }

[artifacts.hello]
kind = "file"
source = "artifacts/hello.txt.tera"
template = true
validate_as = "none"

[artifacts.bye]
kind = "file"
source = "artifacts/bye.txt.tera"
template = false
validate_as = "none"

[targets.hello_copy]
artifact = "hello"
profile = "base"
target_path = "linked/hello.txt"
method = "copy"

[env.emit.default]
vars = [
  { key = "OPENAI_API_KEY", binding = { kind = "env", var = "OPENAI_API_KEY" } }
]

[scripts.sync]
kind = "shell"
entry = "python3"
args = [
  "-c",
  '''import os,sys; payload=sys.stdin.read().strip(); print(f"sync:{os.environ.get('OPENAI_API_KEY', 'missing')}:{payload}")'''
]
cwd_policy = "workspace"
env = [
  { key = "OPENAI_API_KEY", binding = { kind = "env", var = "OPENAI_API_KEY" } }
]
stdin_mode = "text"
stdout_mode = "capture"
stderr_mode = "capture"
expected_exit_codes = [0]

[mcp_servers.local]
transport = { kind = "stdio", command = "npx", args = ["-y", "@modelcontextprotocol/server-filesystem", "."] }
env = [
  { key = "OPENAI_API_KEY", binding = { kind = "env", var = "OPENAI_API_KEY" } }
]
`
  );

  await writeFile(path.join(workspaceRoot, 'artifacts', 'hello.txt.tera'), 'Hello {{ name }}!\n');
  await writeFile(path.join(workspaceRoot, 'artifacts', 'bye.txt.tera'), 'Bye from AgentStow.\n');

  await runCommand('git', ['init'], workspaceRoot);
  await runCommand('git', ['config', 'user.name', 'AgentStow Playwright'], workspaceRoot);
  await runCommand('git', ['config', 'user.email', 'playwright@agentstow.local'], workspaceRoot);
  await runCommand('git', ['add', '.'], workspaceRoot);
  await runCommand('git', ['commit', '-m', 'initial workspace'], workspaceRoot);

  await writeFile(
    path.join(workspaceRoot, 'artifacts', 'hello.txt.tera'),
    'Hello {{ name }} from Git!\n'
  );
  await runCommand('git', ['add', '.'], workspaceRoot);
  await runCommand('git', ['commit', '-m', 'update hello template'], workspaceRoot);

  return workspaceRoot;
}

export async function goToWorkspaceBoot(page: Page): Promise<void> {
  await page.goto('/');

  const bootRegion = page.getByRole('region', { name: 'Workspace 引导' });
  if (await bootRegion.isVisible().catch(() => false)) {
    await expect(bootRegion).toBeVisible();
    return;
  }

  await page.getByRole('button', { name: 'Workspace', exact: true }).click();
  await expect(bootRegion).toBeVisible();
}

export async function openWorkspace(
  page: Page,
  workspaceRoot = PLAYWRIGHT_WORKSPACE_ROOT
): Promise<void> {
  await goToWorkspaceBoot(page);
  await page.getByRole('textbox', { name: 'Workspace 路径' }).fill(workspaceRoot);
  await page.getByRole('button', { name: '检查路径', exact: true }).click();
  await expect(page.getByTestId('workspace-probe-summary')).toContainText(workspaceRoot);
  await page.getByRole('button', { name: '打开 workspace', exact: true }).click();
  await expect(page.getByTestId('artifact-tree-item:hello')).toBeVisible();
  await expect(page.getByTestId('artifact-source-editor').locator('.cm-content')).toBeVisible();
}
