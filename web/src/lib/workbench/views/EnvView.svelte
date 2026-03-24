<script lang="ts">
  import CodeEditor from '$lib/components/CodeEditor.svelte';
  import SplitView from '$lib/components/SplitView.svelte';
  import type {
    EnvEmitResponse,
    EnvEmitSetSummaryResponse,
    EnvUsageRefResponse,
    ShellKindResponse
  } from '$lib/types';
  import type { ManifestInsertKind } from '$lib/workbench/manifest_snippets';

  type Props = {
    envSets: EnvEmitSetSummaryResponse[];
    selectedEnvSet: string | null;
    activeEnvSet: EnvEmitSetSummaryResponse | null;
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
    onOpenUsageRef: (ref: EnvUsageRefResponse) => void;
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
    onOpenUsageRef,
    onOpenManifestEditor,
    onCreateManifestObject
  }: Props = $props();

  const envObjectPreview = $derived.by(() =>
    JSON.stringify(
      Object.fromEntries(
        (activeEnvSet?.vars ?? []).map((binding) => [binding.key, binding.rendered_placeholder])
      ),
      null,
      2
    )
  );

  const bindingGuideSnippet = $derived.by(() => {
    const envSetId = activeEnvSet?.id ?? 'default';
    const key = activeEnvSet?.vars[0]?.key ?? 'OPENAI_API_KEY';

    return `[env.emit.${envSetId}]
vars = [
  { key = "${key}", binding = { kind = "env", var = "${key}" } },
  { key = "INLINE_EXAMPLE", binding = { kind = "literal", value = "replace-me" } }
]`;
  });

  function refKindLabel(ref: EnvUsageRefResponse): string {
    if (ref.owner_kind === 'env_emit_set') {
      return 'env';
    }
    if (ref.owner_kind === 'script') {
      return 'script';
    }
    return 'mcp';
  }
</script>

<SplitView autoSaveId="workbench:view:env" initialLeftPct={22} minLeftPx={256} minRightPx={760}>
  {#snippet left()}
    <aside class="explorer surface" aria-label="资源面板">
      <div class="explorer__head">
        <p class="explorer__eyebrow">ENV</p>
        <p class="explorer__hint">环境导出集会把绑定后的变量渲染成 shell 可执行脚本。</p>
      </div>

  <div class="explorer__section">
    <div class="section__title">
      <span>Export Sets</span>
      <strong>{envSets.length}</strong>
    </div>
    <div class="chips chips--tight" aria-label="Env export set actions">
      <button class="chip" onclick={() => onCreateManifestObject('env_set')} type="button">
        新建导出集
      </button>
      <button class="chip" onclick={onOpenManifestEditor} type="button">
        编辑 manifest
      </button>
    </div>
    <ul class="list">
      {#if envSets.length === 0}
        <li class="list__static">
          <span class="muted">（未声明环境导出集）</span>
          <span class="mono">env.emit</span>
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
              <span class="list__meta">
                {envSet.available_count}/{envSet.vars.length} ready{envSet.missing_count ? ` · ${envSet.missing_count} missing` : ''}
              </span>
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
      <strong>{activeEnvSet?.id ?? '未选择环境导出集'}</strong>
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
              当前导出集会按所选 shell 渲染激活脚本，便于直接复制或在终端中执行。
            </p>
            {#if errorMessage}
              <p class="notice notice--error">{errorMessage}</p>
            {:else if statusLine}
              <p class="stack-note">{statusLine}</p>
            {/if}
            <div class="panel__body panel__body--flush">
              {#if !activeEnvSet}
                <p class="empty empty--flush">（暂无环境导出集，可先在 manifest 中声明）</p>
              {:else}
                {#key envScript?.text ?? ''}
                  <CodeEditor value={envScript?.text ?? ''} readonly={true} documentLanguage="shell" />
                {/key}
              {/if}
            </div>
          </div>
        </section>
      {/snippet}

      {#snippet right()}
        <section class="region secondary-sidebar" aria-label="变量检查器">
          <div class="region__header">
            <span>Variables</span>
            <span class="mono">
              {activeEnvSet ? `${activeEnvSet.available_count}/${activeEnvSet.vars.length} ready` : '0'}
            </span>
          </div>

          <div class="region__body">
            {#if !activeEnvSet}
              <p class="empty empty--flush">（选择环境导出集后可查看变量绑定）</p>
            {:else if activeEnvSet.vars.length === 0}
              <p class="empty empty--flush">（该环境导出集暂无变量）</p>
            {:else}
              <div class="inspector-section">
                <div class="section__title">
                  <span>Health</span>
                  <strong>{activeEnvSet.missing_count === 0 ? 'ready' : 'attention'}</strong>
                </div>
                <div class="subject-summary">
                  <div class="summary-row">
                    <span class="summary-row__label">Ready</span>
                    <span class="summary-row__value mono">{activeEnvSet.available_count}</span>
                  </div>
                  <div class="summary-row">
                    <span class="summary-row__label">Missing</span>
                    <span class="summary-row__value mono">{activeEnvSet.missing_count}</span>
                  </div>
                  <div class="summary-row">
                    <span class="summary-row__label">Refs</span>
                    <span class="summary-row__value mono">{activeEnvSet.referrers.length}</span>
                  </div>
                </div>
              </div>

              <div class="inspector-section">
                <div class="section__title">
                  <span>Binding Modes</span>
                  <strong>env / literal</strong>
                </div>
                <p class="stack-note">
                  `env` 会读取启动 `agentstow serve` 的宿主进程环境；如果你在服务启动后才修改环境变量，需要重启服务才能重新探测。
                </p>
                <p class="stack-note">
                  `literal` 会把值直接写进 manifest，适合开发期快速验证；当前不会自动加载 workspace 下的 `.env` 文件。
                </p>
                <div class="panel__body panel__body--flush">
                  {#key bindingGuideSnippet}
                    <CodeEditor
                      value={bindingGuideSnippet}
                      readonly={true}
                      documentLanguage="toml"
                      testId="env-binding-guide"
                    />
                  {/key}
                </div>
                <div class="chips chips--tight">
                  <button
                    class="chip"
                    type="button"
                    onclick={() => void onCopyToClipboard(bindingGuideSnippet, 'env binding snippet')}
                  >
                    复制 binding 示例
                  </button>
                </div>
              </div>

              <div class="inspector-section">
                <div class="section__title">
                  <span>Bindings</span>
                  <strong>{activeEnvSet.vars.length}</strong>
                </div>
                <ul class="result-list" aria-label="Env bindings">
                  {#each activeEnvSet.vars as v (v.key)}
                    <li class="result-row">
                      <span class={['pill', v.available ? 'pill--ok' : 'pill--warn'].join(' ')}>
                        {v.available ? 'ready' : 'missing'}
                      </span>
                      <div class="result-row__main">
                        <span class="result-row__title">{v.key}</span>
                        <span class="result-row__detail mono">
                          {v.binding_kind} · {v.binding} · {v.rendered_placeholder}
                        </span>
                        {#if v.source_env_var}
                          <span class="result-row__detail mono">source env: {v.source_env_var}</span>
                        {/if}
                        {#if v.diagnostic}
                          <span class="result-row__detail">{v.diagnostic}</span>
                        {/if}
                        {#if v.referrers.length > 0}
                          <div class="chips chips--tight" aria-label={`Env referrers for ${v.key}`}>
                            {#each v.referrers as ref (`${v.key}:${ref.owner_kind}:${ref.owner_id}`)}
                              <button class="chip" type="button" onclick={() => onOpenUsageRef(ref)}>
                                {ref.label}
                              </button>
                            {/each}
                          </div>
                        {/if}
                      </div>
                    </li>
                  {/each}
                </ul>
              </div>

              <div class="inspector-section">
                <div class="section__title">
                  <span>Placeholder Preview</span>
                  <strong>masked</strong>
                </div>
                <p class="stack-note">这里只展示渲染占位，不回显真实 secret 值。</p>
                <div class="panel__body panel__body--flush">
                  {#key envObjectPreview}
                    <CodeEditor
                      value={envObjectPreview}
                      readonly={true}
                      documentLanguage="json"
                      testId="env-object-preview"
                    />
                  {/key}
                </div>
                <div class="chips chips--tight">
                  <button
                    class="chip"
                    disabled={activeEnvSet.vars.length === 0}
                    type="button"
                    onclick={() => void onCopyToClipboard(envObjectPreview, 'env object')}
                  >
                    复制 preview JSON
                  </button>
                </div>
              </div>

              <div class="inspector-section">
                <div class="section__title">
                  <span>Usage</span>
                  <strong>{activeEnvSet.referrers.length}</strong>
                </div>
                {#if activeEnvSet.referrers.length === 0}
                  <p class="empty empty--flush">（当前导出集尚未被 script 或 MCP 引用）</p>
                {:else}
                  <div class="token-action-list" data-testid="env-referrer-list">
                    {#each activeEnvSet.referrers as ref (`${ref.owner_kind}:${ref.owner_id}`)}
                      <div class="token-action-row">
                        <button
                          class="token token--interactive token--object"
                          type="button"
                          onclick={() => onOpenUsageRef(ref)}
                        >
                          <span>{ref.label}</span>
                          <span class="token__meta">{refKindLabel(ref)} · {ref.owner_id}</span>
                        </button>
                        <button
                          class="ui-button ui-button--ghost ui-button--icon"
                          type="button"
                          onclick={() => void onCopyToClipboard(ref.owner_id, `${refKindLabel(ref)} id`)}
                        >
                          复制
                        </button>
                      </div>
                    {/each}
                  </div>
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
