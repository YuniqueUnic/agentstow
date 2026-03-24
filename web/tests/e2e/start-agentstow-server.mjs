import { spawn } from 'node:child_process';
import { mkdir, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const currentDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(currentDir, '..', '..', '..');
const port = Number(process.env.PLAYWRIGHT_AGENTSTOW_PORT ?? '8877');
const workspaceRoot = path.join(os.tmpdir(), 'agentstow-playwright-workspace');

async function runCommand(command, args, cwd) {
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
        resolve();
        return;
      }
      reject(new Error(`${command} ${args.join(' ')} failed (${code}): ${stderr}`));
    });
  });
}

async function prepareWorkspace() {
  await rm(workspaceRoot, { recursive: true, force: true });
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
entry = "echo"
args = ["sync"]
env = [
  { key = "OPENAI_API_KEY", binding = { kind = "env", var = "OPENAI_API_KEY" } }
]

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
}

await prepareWorkspace();

const server = spawn(
  'cargo',
  ['run', '-p', 'agentstow-cli', '--', 'serve', '--addr', `127.0.0.1:${port}`],
  {
    cwd: repoRoot,
    stdio: 'inherit',
    env: {
      ...process.env,
      OPENAI_API_KEY: process.env.OPENAI_API_KEY ?? 'agentstow-playwright-token'
    }
  }
);

for (const signal of ['SIGINT', 'SIGTERM']) {
  process.on(signal, () => {
    server.kill(signal);
  });
}

server.on('exit', (code) => {
  process.exit(code ?? 0);
});
