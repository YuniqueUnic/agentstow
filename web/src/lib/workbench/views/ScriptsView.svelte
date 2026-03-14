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
    errorMessage,
    statusLine,
    onSelectScript,
    onScriptStdin,
    onScriptRun,
    onCopyToClipboard,
    onOpenManifestEditor,
    onCreateManifestObject
  }: Props = $props();

  let outputTab = $state<'stdout' | 'stderr' | 'meta'>('stdout');
</script>

<aside class="explorer surface" aria-label="资源面板">
  <div class="explorer__head">
    <p class="explorer__eyebrow">SCRIPTS</p>
    <p class="explorer__hint">中心区编辑 stdin，底部 panel 看运行输出。</p>
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
        disabled={!scriptRun?.stdout}
        type="button"
        onclick={() => void onCopyToClipboard(scriptRun?.stdout ?? '', 'stdout')}
      >
        复制 stdout
      </button>
    </div>
  </div>

  {#if errorMessage}
    <p class="notice notice--error">{errorMessage}</p>
  {/if}
  <p class="status-line" aria-live="polite">{statusLine}</p>

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
                <span>stdin / request</span>
                <span class="mono">{activeScript?.kind ?? 'script'}</span>
              </div>
              <div class="panel__body panel__body--flush">
                {#if !activeScript}
                  <p class="empty empty--flush">（选择 script 后可输入 stdin 并运行）</p>
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
                    <Tabs.Trigger class="tab" value="stdout">stdout</Tabs.Trigger>
                    <Tabs.Trigger class="tab" value="stderr">stderr</Tabs.Trigger>
                    <Tabs.Trigger class="tab" value="meta">meta</Tabs.Trigger>
                  </Tabs.List>
                  <span class="mono">exit={scriptRun?.exit_code ?? 'idle'}</span>
                </div>

                <Tabs.Content class="panel__body" value="stdout">
                  <div class="terminal">
                    <pre class="terminal__screen">{scriptRun?.stdout ?? '（无输出或尚未运行）'}</pre>
                  </div>
                </Tabs.Content>

                <Tabs.Content class="panel__body" value="stderr">
                  <div class="terminal">
                    <pre class="terminal__screen terminal__screen--stderr">
                      {scriptRun?.stderr ?? '（无错误输出或尚未运行）'}
                    </pre>
                  </div>
                </Tabs.Content>

                <Tabs.Content class="panel__body" value="meta">
                  {#if !activeScript}
                    <p class="empty empty--flush">（暂无 script 元信息）</p>
                  {:else}
                    <div class="inspector-table">
                      <div class="inspector-row">
                        <span class="inspector-row__label">Entry</span>
                        <span class="inspector-row__value inspector-row__value--mono">{activeScript.entry}</span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">Args</span>
                        <span class="inspector-row__value inspector-row__value--mono">
                          {activeScript.args.join(' ') || '（无）'}
                        </span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">Timeout</span>
                        <span class="inspector-row__value inspector-row__value--mono">
                          {activeScript.timeout_ms ?? '（无）'}
                        </span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">Result</span>
                        <span class="inspector-row__value inspector-row__value--mono">
                          exit={scriptRun?.exit_code ?? '（未运行）'}
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
            <span>Script Inspector</span>
            <button
              class="ui-button ui-button--ghost"
              disabled={!activeScript}
              type="button"
              onclick={() => void onCopyToClipboard(activeScript?.entry ?? '', 'entry')}
            >
              复制 entry
            </button>
          </div>

          <div class="panel__body panel__body--flush">
            {#if !activeScript}
              <p class="empty empty--flush">（选择 script 后查看 entry / args / env keys）</p>
            {:else}
              <div class="inspector-table">
                <div class="inspector-row">
                  <span class="inspector-row__label">Kind</span>
                  <span class="inspector-row__value inspector-row__value--mono">{activeScript.kind}</span>
                </div>
                <div class="inspector-row">
                  <span class="inspector-row__label">Entry</span>
                  <span class="inspector-row__value inspector-row__value--mono">{activeScript.entry}</span>
                </div>
                <div class="inspector-row">
                  <span class="inspector-row__label">Args</span>
                  <span class="inspector-row__value inspector-row__value--mono">
                    {activeScript.args.join(' ') || '（无）'}
                  </span>
                </div>
                <div class="inspector-row">
                  <span class="inspector-row__label">Env Keys</span>
                  <span class="inspector-row__value inspector-row__value--mono">
                    {activeScript.env_keys.join(', ') || '（无）'}
                  </span>
                </div>
                <div class="inspector-row">
                  <span class="inspector-row__label">Timeout</span>
                  <span class="inspector-row__value inspector-row__value--mono">
                    {activeScript.timeout_ms ?? '（无）'}
                  </span>
                </div>
              </div>
            {/if}
          </div>
        </section>
      {/snippet}
    </SplitView>
  </div>
</main>
