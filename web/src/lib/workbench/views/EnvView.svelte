<script lang="ts">
  import CodeEditor from '$lib/components/CodeEditor.svelte';
  import SplitView from '$lib/components/SplitView.svelte';
  import type { EnvEmitResponse, EnvSetSummaryResponse, ShellKindResponse } from '$lib/types';
  import type { ManifestInsertKind } from '$lib/workbench/manifest_snippets';

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
    onOpenManifestEditor: () => void;
    onCreateManifestObject: (kind: ManifestInsertKind) => void;
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
    onCopyToClipboard,
    onOpenManifestEditor,
    onCreateManifestObject
  }: Props = $props();
</script>

<SplitView autoSaveId="workbench:view:env" initialLeftPct={22} minLeftPx={256} minRightPx={760}>
  {#snippet left()}
    <aside class="explorer surface" aria-label="资源面板">
      <div class="explorer__head">
        <p class="explorer__eyebrow">ENV</p>
        <p class="explorer__hint">环境变量是工作台对象，不是独立后台页。</p>
      </div>

  <div class="explorer__section">
    <div class="section__title">
      <span>Env Sets</span>
      <strong>{envSets.length}</strong>
    </div>
    <div class="chips chips--tight" aria-label="Env set actions">
      <button class="chip" onclick={() => onCreateManifestObject('env_set')} type="button">
        新建 env set
      </button>
      <button class="chip" onclick={onOpenManifestEditor} type="button">
        编辑 manifest
      </button>
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
  {/snippet}

  {#snippet right()}
    <main class="canvas" aria-label="工作区画布">
  <div class="canvas__head">
    <div class="title">
      <strong>{activeEnvSet?.id ?? '未选择 env set'}</strong>
      <span class="muted">· shell preview</span>
    </div>

    <div class="canvas__actions">
      <button class="ui-button ui-button--subtle" type="button" onclick={onOpenManifestEditor}>
        编辑 manifest
      </button>
      <button
        class="ui-button ui-button--ghost"
        disabled={!selectedEnvSet || busyEnvEmit}
        type="button"
        onclick={() => void onEnvEmit()}
      >
        {busyEnvEmit ? '生成中…' : '生成脚本'}
      </button>
      <button
        class="ui-button ui-button--primary"
        disabled={!envScript?.text}
        type="button"
        onclick={() => void onCopyToClipboard(envScript?.text ?? '', '脚本')}
      >
        复制脚本
      </button>
    </div>
  </div>

  {#if errorMessage}
    <p class="notice notice--error">{errorMessage}</p>
  {/if}
  <p class="status-line" aria-live="polite">{statusLine}</p>

  <div class="split surface">
    <SplitView autoSaveId="workbench:env:shell" initialLeftPct={68} minLeftPx={420} minRightPx={280}>
      {#snippet left()}
        <section class="region" aria-label="Shell 预览">
          <div class="region__header">
            <span>Shell Preview</span>
            <div class="region__toolbar" aria-label="Shell 选择">
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
          </div>

          <div class="region__body region__body--stack">
            <p class="stack-note">
              当前 env set 会按所选 shell 渲染激活脚本，便于直接复制或在终端中执行。
            </p>
            <div class="panel__body panel__body--flush">
              {#if !activeEnvSet}
                <p class="empty empty--flush">（暂无 env set，可先在 manifest 中声明）</p>
              {:else}
                <CodeEditor value={envScript?.text ?? ''} readonly={true} />
              {/if}
            </div>
          </div>
        </section>
      {/snippet}

      {#snippet right()}
        <section class="region secondary-sidebar" aria-label="变量检查器">
          <div class="region__header">
            <span>Variables</span>
            <span class="mono">{activeEnvSet?.vars.length ?? 0} keys</span>
          </div>

          <div class="panel__body panel__body--flush">
            {#if !activeEnvSet}
              <p class="empty empty--flush">（选择 env set 后可查看变量绑定）</p>
            {:else if activeEnvSet.vars.length === 0}
              <p class="empty empty--flush">（该 env set 暂无变量）</p>
            {:else}
              <div class="inspector-table">
                {#each activeEnvSet.vars as v (v.key)}
                  <div class="inspector-row">
                    <span class="inspector-row__label">{v.key}</span>
                    <span class="inspector-row__value inspector-row__value--mono">{v.binding}</span>
                  </div>
                {/each}
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
