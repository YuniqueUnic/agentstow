<script lang="ts">
  import { Tabs } from 'bits-ui';

  import CodeEditor from '$lib/components/CodeEditor.svelte';
  import SplitView from '$lib/components/SplitView.svelte';
  import type { ScriptRunResponse, ScriptSummaryResponse } from '$lib/types';
  import type { ManifestInsertKind } from '$lib/workbench/manifest_snippets';

  type Props = {
    scripts: ScriptSummaryResponse[];
    selectedScript: string | null;
    activeScript: ScriptSummaryResponse | null;
    scriptStdin: string;
    scriptRun: ScriptRunResponse | null;
    busyScriptRun: boolean;
    errorMessage: string | null;
    statusLine: string;
    onSelectScript: (id: string) => void;
    onScriptStdin: (next: string) => void;
    onScriptRun: () => Promise<void>;
    onCopyToClipboard: (text: string, label: string) => Promise<void>;
    onOpenManifestEditor: () => void;
    onCreateManifestObject: (kind: ManifestInsertKind) => void;
  };

  let {
    scripts,
    selectedScript,
    activeScript,
    scriptStdin,
    scriptRun,
    busyScriptRun,
    onSelectScript,
    onScriptStdin,
    onScriptRun,
    onCopyToClipboard,
    onOpenManifestEditor,
    onCreateManifestObject
  }: Props = $props();

  let outputTab = $state<'stdout' | 'stderr' | 'summary'>('summary');

  const scriptSupportsStdin = $derived(activeScript ? activeScript.stdin_mode !== 'none' : false);
  const capturesStdout = $derived(activeScript ? activeScript.stdout_mode !== 'passthrough' : false);
  const capturesStderr = $derived(activeScript ? activeScript.stderr_mode !== 'passthrough' : false);
  const commandPreview = $derived(
    activeScript
      ? [activeScript.entry, ...activeScript.args].filter(Boolean).join(' ')
      : ''
  );

  function modeLabel(mode: 'none' | 'text' | 'json' | 'passthrough' | 'capture'): string {
    switch (mode) {
      case 'none':
        return 'disabled';
      case 'text':
        return 'text';
      case 'json':
        return 'json';
      case 'capture':
        return 'capture';
      case 'passthrough':
      default:
        return 'passthrough';
    }
  }

  $effect(() => {
    const allowedTabs = [
      capturesStdout ? 'stdout' : null,
      capturesStderr ? 'stderr' : null,
      'summary'
    ].filter((tab): tab is typeof outputTab => tab !== null);

    if (!allowedTabs.includes(outputTab)) {
      outputTab = allowedTabs[0] ?? 'summary';
    }
  });
</script>

<SplitView autoSaveId="workbench:view:scripts" initialLeftPct={22} minLeftPx={256} minRightPx={760}>
  {#snippet left()}
    <aside class="explorer surface" aria-label="资源面板">
      <div class="explorer__head">
        <p class="explorer__eyebrow">SCRIPTS</p>
        <p class="explorer__hint">按脚本契约展示 stdin / stdout / stderr，避免把所有脚本都伪装成通用终端。</p>
      </div>

  <div class="explorer__section">
    <div class="section__title">
      <span>Scripts</span>
      <strong>{scripts.length}</strong>
    </div>
    <div class="chips chips--tight" aria-label="Scripts actions">
      <button class="chip" onclick={() => onCreateManifestObject('script')} type="button">
        新建 script
      </button>
      <button class="chip" onclick={onOpenManifestEditor} type="button">
        编辑 manifest
      </button>
    </div>
    <ul class="list">
      {#if scripts.length === 0}
        <li class="list__static">
          <span class="muted">（未声明 scripts）</span>
          <span class="mono">scripts</span>
        </li>
      {:else}
        {#each scripts as script (script.id)}
          <li>
            <button
              class={[
                'list__item',
                selectedScript === script.id ? 'list__item--active' : ''
              ].join(' ')}
              onclick={() => onSelectScript(script.id)}
              type="button"
            >
              <span class="list__dot" aria-hidden="true"></span>
              <span class="list__name">{script.id}</span>
              <span class="list__meta">{script.kind}</span>
            </button>
          </li>
        {/each}
      {/if}
    </ul>
  </div>
    </aside>
  {/snippet}

  {#snippet right()}
    <main class="canvas" aria-label="工作区画布">
  <div class="canvas__head">
    <div class="title">
      <strong>{activeScript?.id ?? '未选择 script'}</strong>
      <span class="muted">{activeScript ? `· ${activeScript.kind}` : '· terminal workflow'}</span>
    </div>

    <div class="canvas__actions">
      <button class="ui-button ui-button--subtle" type="button" onclick={onOpenManifestEditor}>
        编辑 manifest
      </button>
      <button
        class="ui-button ui-button--ghost"
        disabled={!selectedScript || busyScriptRun}
        type="button"
        onclick={() => void onScriptRun()}
      >
        {busyScriptRun ? '执行中…' : '运行'}
      </button>
      <button
        class="ui-button ui-button--primary"
        disabled={!commandPreview}
        type="button"
        onclick={() => void onCopyToClipboard(commandPreview, 'command preview')}
      >
        复制命令
      </button>
    </div>
  </div>
  <div class="split surface">
    <SplitView autoSaveId="workbench:scripts:shell" initialLeftPct={66} minLeftPx={460} minRightPx={300}>
      {#snippet left()}
        <SplitView
          autoSaveId="workbench:scripts:stack"
          direction="vertical"
          initialLeftPct={56}
          minLeftPx={260}
          minRightPx={180}
        >
          {#snippet left()}
            <section class="region" aria-label="stdin editor">
              <div class="region__header">
                <span>{scriptSupportsStdin ? 'stdin / request' : 'run contract'}</span>
                <span class="mono">
                  {activeScript ? `${modeLabel(activeScript.stdin_mode)} stdin` : 'script'}
                </span>
              </div>
              <div class="panel__body panel__body--flush">
                {#if !activeScript}
                  <p class="empty empty--flush">（选择 script 后可输入 stdin 并运行）</p>
                {:else if !scriptSupportsStdin}
                  <div class="region__body region__body--stack">
                    <p class="stack-note">
                      当前脚本未声明 `stdin_mode`，因此不会向子进程写入输入流。直接点击“运行”即可按 manifest 契约执行。
                    </p>
                    <div class="inspector-table">
                      <div class="inspector-row">
                        <span class="inspector-row__label">Command</span>
                        <span class="inspector-row__value inspector-row__value--mono">
                          {commandPreview || activeScript.entry}
                        </span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">stdin</span>
                        <span class="inspector-row__value inspector-row__value--mono">
                          {modeLabel(activeScript.stdin_mode)}
                        </span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">stdout</span>
                        <span class="inspector-row__value inspector-row__value--mono">
                          {modeLabel(activeScript.stdout_mode)}
                        </span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">stderr</span>
                        <span class="inspector-row__value inspector-row__value--mono">
                          {modeLabel(activeScript.stderr_mode)}
                        </span>
                      </div>
                    </div>
                  </div>
                {:else}
                  <CodeEditor value={scriptStdin} onChange={onScriptStdin} />
                {/if}
              </div>
            </section>
          {/snippet}

          {#snippet right()}
            <section class="panel bottom-panel" aria-label="脚本输出面板">
              <Tabs.Root value={outputTab} onValueChange={(next) => (outputTab = next as typeof outputTab)}>
                <div class="region__header">
                  <Tabs.List class="tabs" aria-label="Script output tabs">
                    {#if capturesStdout}
                      <Tabs.Trigger class="tab" value="stdout">stdout</Tabs.Trigger>
                    {/if}
                    {#if capturesStderr}
                      <Tabs.Trigger class="tab" value="stderr">stderr</Tabs.Trigger>
                    {/if}
                    <Tabs.Trigger class="tab" value="summary">summary</Tabs.Trigger>
                  </Tabs.List>
                  <span class="mono">exit={scriptRun?.exit_code ?? 'idle'}</span>
                </div>

                {#if capturesStdout}
                  <Tabs.Content class="panel__body" value="stdout">
                    <div class="terminal">
                      <pre class="terminal__screen">{scriptRun?.stdout ?? '（stdout 未捕获或尚未运行）'}</pre>
                    </div>
                  </Tabs.Content>
                {/if}

                {#if capturesStderr}
                  <Tabs.Content class="panel__body" value="stderr">
                    <div class="terminal">
                      <pre class="terminal__screen terminal__screen--stderr">
                        {scriptRun?.stderr ?? '（stderr 未捕获或尚未运行）'}
                      </pre>
                    </div>
                  </Tabs.Content>
                {/if}

                <Tabs.Content class="panel__body" value="summary">
                  {#if !activeScript}
                    <p class="empty empty--flush">（暂无 script 元信息）</p>
                  {:else}
                    <div class="inspector-table">
                      <div class="inspector-row">
                        <span class="inspector-row__label">Command</span>
                        <span class="inspector-row__value inspector-row__value--mono">{commandPreview}</span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">CWD</span>
                        <span class="inspector-row__value inspector-row__value--mono">
                          {activeScript.cwd_policy}
                        </span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">stdio</span>
                        <span class="inspector-row__value inspector-row__value--mono">
                          stdin={modeLabel(activeScript.stdin_mode)} · stdout={modeLabel(activeScript.stdout_mode)} · stderr={modeLabel(activeScript.stderr_mode)}
                        </span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">Expected Exit</span>
                        <span class="inspector-row__value inspector-row__value--mono">
                          {activeScript.expected_exit_codes.length > 0
                            ? activeScript.expected_exit_codes.join(', ')
                            : '0'}
                        </span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">Timeout</span>
                        <span class="inspector-row__value inspector-row__value--mono">
                          {activeScript.timeout_ms ?? '（无）'}
                        </span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">Last Exit</span>
                        <span class="inspector-row__value inspector-row__value--mono">
                          {scriptRun?.exit_code ?? '（未运行）'}
                        </span>
                      </div>
                    </div>
                  {/if}
                </Tabs.Content>
              </Tabs.Root>
            </section>
          {/snippet}
        </SplitView>
      {/snippet}

      {#snippet right()}
        <section class="region secondary-sidebar" aria-label="脚本检查器">
          <div class="region__header">
            <span>Script Contract</span>
            <button
              class="ui-button ui-button--ghost"
              disabled={!commandPreview}
              type="button"
              onclick={() => void onCopyToClipboard(commandPreview, 'command preview')}
            >
              复制命令
            </button>
          </div>

          <div class="region__body">
            {#if !activeScript}
              <p class="empty empty--flush">（选择 script 后查看执行契约与 env 依赖）</p>
            {:else}
              <div class="inspector-section">
                <div class="section__title">
                  <span>Execution</span>
                  <strong>{activeScript.kind}</strong>
                </div>
                <div class="subject-summary">
                  <div class="summary-row">
                    <span class="summary-row__label">stdin</span>
                    <span class="summary-row__value mono">{modeLabel(activeScript.stdin_mode)}</span>
                  </div>
                  <div class="summary-row">
                    <span class="summary-row__label">stdout</span>
                    <span class="summary-row__value mono">{modeLabel(activeScript.stdout_mode)}</span>
                  </div>
                  <div class="summary-row">
                    <span class="summary-row__label">stderr</span>
                    <span class="summary-row__value mono">{modeLabel(activeScript.stderr_mode)}</span>
                  </div>
                  <div class="summary-row">
                    <span class="summary-row__label">cwd</span>
                    <span class="summary-row__value mono">{activeScript.cwd_policy}</span>
                  </div>
                </div>
              </div>

              <div class="inspector-section">
                <div class="section__title">
                  <span>Env Bindings</span>
                  <strong>{activeScript.env_bindings.length}</strong>
                </div>
                {#if activeScript.env_bindings.length === 0}
                  <p class="empty empty--flush">（该 script 未声明 env 依赖）</p>
                {:else}
                  <ul class="result-list" aria-label="Script env bindings">
                    {#each activeScript.env_bindings as binding (binding.key)}
                      <li class="result-row">
                        <span class={['pill', binding.available ? 'pill--ok' : 'pill--warn'].join(' ')}>
                          {binding.available ? 'ready' : 'missing'}
                        </span>
                        <div class="result-row__main">
                          <span class="result-row__title">{binding.key}</span>
                          <span class="result-row__detail mono">
                            {binding.binding_kind} · {binding.binding} · {binding.rendered_placeholder}
                          </span>
                          {#if binding.diagnostic}
                            <span class="result-row__detail">{binding.diagnostic}</span>
                          {/if}
                        </div>
                      </li>
                    {/each}
                  </ul>
                {/if}
              </div>
            {/if}
          </div>
        </section>
      {/snippet}
    </SplitView>
  </div>
    </main>
  {/snippet}
</SplitView>
