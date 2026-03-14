<script lang="ts">
  import CodeEditor from '$lib/components/CodeEditor.svelte';
  import SplitView from '$lib/components/SplitView.svelte';
  import type { EnvEmitResponse, EnvSetSummaryResponse, ShellKindResponse } from '$lib/types';

  type Props = {
    envSets: EnvSetSummaryResponse[];
    selectedEnvSet: string | null;
    activeEnvSet: EnvSetSummaryResponse | null;
    selectedShell: ShellKindResponse;
    shellChoices: ShellKindResponse[];
    envScript: EnvEmitResponse | null;
    busyEnvEmit: boolean;
    errorMessage: string | null;
    statusLine: string;
    onSelectEnvSet: (id: string) => void;
    onSelectShell: (shell: ShellKindResponse) => void;
    onEnvEmit: () => Promise<void>;
    onCopyToClipboard: (text: string, label: string) => Promise<void>;
  };

  let {
    envSets,
    selectedEnvSet,
    activeEnvSet,
    selectedShell,
    shellChoices,
    envScript,
    busyEnvEmit,
    errorMessage,
    statusLine,
    onSelectEnvSet,
    onSelectShell,
    onEnvEmit,
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
    <p class="explorer__eyebrow">ENV</p>
    <p class="explorer__hint">选择 env set 后生成 shell 激活脚本</p>
  </div>

  <div class="explorer__section">
    <div class="section__title">
      <span>Env Sets</span>
      <strong>{envSets.length}</strong>
    </div>
    <ul class="list">
      {#if envSets.length === 0}
        <li class="list__static">
          <span class="muted">（未声明 env sets）</span>
          <span class="mono">env_sets</span>
        </li>
      {:else}
        {#each envSets as envSet (envSet.id)}
          <li>
            <button
              class={[
                'list__item',
                selectedEnvSet === envSet.id ? 'list__item--active' : ''
              ].join(' ')}
              onclick={() => onSelectEnvSet(envSet.id)}
              type="button"
            >
              <span class="list__dot" aria-hidden="true"></span>
              <span class="list__name">{envSet.id}</span>
              <span class="list__meta">{envSet.vars.length} vars</span>
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
      <strong>{activeEnvSet?.id ?? '未选择 env set'}</strong>
      <span class="muted">· env</span>
    </div>

    <div class="canvas__actions">
      <md-outlined-button
        disabled={!selectedEnvSet || busyEnvEmit}
        onclick={() => void onEnvEmit()}
        onkeydown={(event) => activateOnKey(event, () => void onEnvEmit())}
        role="button"
        tabindex="0"
      >
        {busyEnvEmit ? '生成中…' : '生成脚本'}
      </md-outlined-button>
      <md-filled-tonal-button
        disabled={!envScript?.text}
        onclick={() => void onCopyToClipboard(envScript?.text ?? '', '脚本')}
        onkeydown={(event) =>
          activateOnKey(event, () => void onCopyToClipboard(envScript?.text ?? '', '脚本'))}
        role="button"
        tabindex="0"
      >
        复制
      </md-filled-tonal-button>
    </div>
  </div>

  {#if errorMessage}
    <p class="notice notice--error">{errorMessage}</p>
  {/if}
  <p class="status-line" aria-live="polite">{statusLine}</p>

  <div class="split surface">
    <SplitView initialLeftPct={40} minLeftPx={320} minRightPx={360}>
      {#snippet left()}
        <div class="pane">
          <div class="pane__title">Variables</div>
          <div class="pane__body">
            {#if !activeEnvSet}
              <p class="muted">（暂无 env set）</p>
            {:else if activeEnvSet.vars.length === 0}
              <p class="muted">（该 env set 没有变量）</p>
            {:else}
              <ul class="kv">
                {#each activeEnvSet.vars as v (v.key)}
                  <li class="kv__row">
                    <span class="kv__key">{v.key}</span>
                    <span class="kv__value">{v.binding}</span>
                  </li>
                {/each}
              </ul>
            {/if}
          </div>
        </div>
      {/snippet}

      {#snippet right()}
        <div class="pane">
          <div class="pane__title">Shell</div>
          <div class="pane__body pane__body--stack">
            <div class="chips chips--tight" aria-label="Shell 选择">
              {#each shellChoices as sh (sh)}
                <button
                  class={['chip', selectedShell === sh ? 'chip--active' : ''].join(' ')}
                  onclick={() => onSelectShell(sh)}
                  type="button"
                >
                  {sh}
                </button>
              {/each}
            </div>

            <div class="output">
              <CodeEditor value={envScript?.text ?? ''} readonly={true} />
            </div>
          </div>
        </div>
      {/snippet}
    </SplitView>
  </div>
</main>

