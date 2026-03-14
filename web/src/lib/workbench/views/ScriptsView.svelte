<script lang="ts">
  import SplitView from '$lib/components/SplitView.svelte';
  import type { ScriptRunResponse, ScriptSummaryResponse } from '$lib/types';

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
    onCopyToClipboard
  }: Props = $props();

  function activateOnKey(event: KeyboardEvent, action: () => void): void {
    if (event.key !== 'Enter' && event.key !== ' ') {
      return;
    }
    event.preventDefault();
    action();
  }
</script>

<aside class="explorer surface" aria-label="资源面板">
  <div class="explorer__head">
    <p class="explorer__eyebrow">SCRIPTS</p>
    <p class="explorer__hint">选择脚本后在右侧执行并查看输出</p>
  </div>

  <div class="explorer__section">
    <div class="section__title">
      <span>Scripts</span>
      <strong>{scripts.length}</strong>
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
              class={['list__item', selectedScript === script.id ? 'list__item--active' : ''].join(' ')}
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
      <span class="muted">{activeScript ? `· ${activeScript.kind}` : ''}</span>
    </div>

    <div class="canvas__actions">
      <md-outlined-button
        disabled={!selectedScript || busyScriptRun}
        onclick={() => void onScriptRun()}
        onkeydown={(event) => activateOnKey(event, () => void onScriptRun())}
        role="button"
        tabindex="0"
      >
        {busyScriptRun ? '执行中…' : '运行'}
      </md-outlined-button>
      <md-filled-tonal-button
        disabled={!scriptRun?.stdout}
        onclick={() => void onCopyToClipboard(scriptRun?.stdout ?? '', 'stdout')}
        onkeydown={(event) =>
          activateOnKey(event, () => void onCopyToClipboard(scriptRun?.stdout ?? '', 'stdout'))}
        role="button"
        tabindex="0"
      >
        复制 stdout
      </md-filled-tonal-button>
    </div>
  </div>

  {#if errorMessage}
    <p class="notice notice--error">{errorMessage}</p>
  {/if}
  <p class="status-line" aria-live="polite">{statusLine}</p>

  <div class="split surface">
    <SplitView initialLeftPct={42} minLeftPx={340} minRightPx={360}>
      {#snippet left()}
        <div class="pane">
          <div class="pane__title">Request</div>
          <div class="pane__body">
            {#if !activeScript}
              <p class="muted">（暂无 script）</p>
            {:else}
              <div class="meta">
                <div class="meta__row">
                  <span class="meta__label">Entry</span>
                  <span class="meta__value mono">{activeScript.entry}</span>
                </div>
                <div class="meta__row">
                  <span class="meta__label">Args</span>
                  <span class="meta__value mono">{activeScript.args.join(' ') || '（无）'}</span>
                </div>
                <div class="meta__row">
                  <span class="meta__label">Env</span>
                  <span class="meta__value mono">{activeScript.env_keys.join(', ') || '（无）'}</span>
                </div>
                <div class="meta__row">
                  <span class="meta__label">Timeout</span>
                  <span class="meta__value mono">{activeScript.timeout_ms ?? '（无）'}</span>
                </div>
              </div>

              <md-outlined-text-field
                label="stdin（可选）"
                placeholder="输入 stdin 内容（会以文本写入）"
                value={scriptStdin}
                oninput={(event) => {
                  const target = event.currentTarget as { value?: string } | null;
                  onScriptStdin(typeof target?.value === 'string' ? target.value : '');
                }}
                supporting-text="仅允许执行 manifest 中声明的脚本。"
              ></md-outlined-text-field>
            {/if}
          </div>
        </div>
      {/snippet}

      {#snippet right()}
        <div class="pane">
          <div class="pane__title">Response</div>
          <div class="pane__body">
            <p class="muted small">exit={scriptRun?.exit_code ?? '（未运行）'}</p>
            <div class="output">
              <div class="output__title">stdout</div>
              <pre class="preview">{scriptRun?.stdout ?? '（无输出或未捕获）'}</pre>
            </div>
            <div class="output output--secondary">
              <div class="output__title">stderr</div>
              <pre class="preview preview--stderr">{scriptRun?.stderr ?? '（无输出或未捕获）'}</pre>
            </div>
          </div>
        </div>
      {/snippet}
    </SplitView>
  </div>
</main>

