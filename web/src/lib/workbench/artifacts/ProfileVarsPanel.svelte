<script lang="ts">
  import type {
    ProfileDetailResponse,
    ProfileVarSyntaxModeResponse,
    ProfileVarUpdateItemRequest
  } from '$lib/types';

  type Props = {
    selectedProfile: string | null;
    detail: ProfileDetailResponse | null;
    busy: boolean;
    error: string | null;
    saving: boolean;
    onSave: (vars: ProfileVarUpdateItemRequest[]) => void | Promise<void>;
    onCopyPlaceholder: (text: string, label: string) => void | Promise<void>;
  };

  let {
    selectedProfile,
    detail,
    busy,
    error,
    saving,
    onSave,
    onCopyPlaceholder
  }: Props = $props();

  let draftRows = $state<ProfileVarUpdateItemRequest[]>([]);
  let syncKey = $state<string | null>(null);

  function buildRows(detail: ProfileDetailResponse | null): ProfileVarUpdateItemRequest[] {
    return (detail?.declared_vars ?? []).map((item) => ({
      key: item.key,
      value_json: item.value_json
    }));
  }

  function syntaxModeLabel(mode: ProfileVarSyntaxModeResponse | undefined): string {
    switch (mode) {
      case 'inline':
        return 'inline';
      case 'mixed':
        return 'mixed';
      case 'vars_object':
        return 'vars namespace';
      default:
        return 'unknown';
    }
  }

  function syntaxModeHint(mode: ProfileVarSyntaxModeResponse | undefined): string {
    switch (mode) {
      case 'inline':
        return '当前 manifest 直接把变量写在 profile 顶层；保存后会统一收束到 vars 命名空间。';
      case 'mixed':
        return '当前 manifest 同时存在顶层变量和 vars 命名空间；保存后会统一收束到 vars 命名空间。';
      case 'vars_object':
        return '当前 manifest 已使用专门的 vars 命名空间。';
      default:
        return '保存时会统一写回到 vars 命名空间，避免变量 key 和 profile 元字段冲突。';
    }
  }

  const declaredRowsSnapshot = $derived(JSON.stringify(buildRows(detail)));
  const dirty = $derived(JSON.stringify(draftRows) !== declaredRowsSnapshot);
  const mergedOnlyRows = $derived.by(() => {
    const declaredKeys = new Set((detail?.declared_vars ?? []).map((item) => item.key));
    return (detail?.merged_vars ?? []).filter((item) => !declaredKeys.has(item.key));
  });

  function replaceRow(index: number, patch: Partial<ProfileVarUpdateItemRequest>): void {
    const next = draftRows.slice();
    next[index] = { ...next[index], ...patch };
    draftRows = next;
  }

  function addRow(): void {
    draftRows = [...draftRows, { key: '', value_json: '""' }];
  }

  function removeRow(index: number): void {
    draftRows = draftRows.filter((_, currentIndex) => currentIndex !== index);
  }

  function resetDraft(): void {
    draftRows = buildRows(detail);
  }

  async function save(): Promise<void> {
    await onSave(
      draftRows.map((row) => ({
        key: row.key.trim(),
        value_json: row.value_json.trim()
      }))
    );
  }

  $effect(() => {
    const nextKey = `${selectedProfile ?? ''}::${declaredRowsSnapshot}`;
    if (nextKey === syncKey) {
      return;
    }

    syncKey = nextKey;
    draftRows = buildRows(detail);
  });
</script>

<div class="inspector-section" data-testid="profile-vars-panel">
  <div class="section__title">
    <span>Profile Vars</span>
    <strong>{selectedProfile ?? '未选择'}</strong>
  </div>

  {#if !selectedProfile}
    <p class="empty empty--flush">（选择 profile 后即可编辑变量）</p>
  {:else if busy}
    <p class="empty empty--flush">读取 profile 变量中…</p>
  {:else if error}
    <p class="empty empty--flush">{error}</p>
  {:else}
    <div class="vars-editor">
      <div class="subject-summary">
        <div class="summary-row">
          <span class="summary-row__label">Current Syntax</span>
          <span class="summary-row__value mono">{syntaxModeLabel(detail?.syntax_mode)}</span>
        </div>
        <div class="summary-row">
          <span class="summary-row__label">Declared</span>
          <span class="summary-row__value mono">{detail?.declared_vars.length ?? 0}</span>
        </div>
        <div class="summary-row">
          <span class="summary-row__label">Merged</span>
          <span class="summary-row__value mono">{detail?.merged_vars.length ?? 0}</span>
        </div>
      </div>

      <p class="stack-note">{syntaxModeHint(detail?.syntax_mode)}</p>

      <div class="vars-editor__rows">
        {#if draftRows.length === 0}
          <p class="empty empty--flush">（当前 profile 还没有显式声明 vars，可直接新增）</p>
        {:else}
          {#each draftRows as row, index (`${index}:${row.key}`)}
            <div class="vars-editor__row">
              <label class="field field--compact">
                <span class="field__label">Key</span>
                <input
                  class="field__input mono"
                  type="text"
                  value={row.key}
                  oninput={(event) => {
                    const target = event.currentTarget as HTMLInputElement | null;
                    replaceRow(index, { key: target?.value ?? '' });
                  }}
                />
              </label>

              <label class="field field--compact">
                <span class="field__label">Value JSON</span>
                <input
                  class="field__input mono"
                  type="text"
                  value={row.value_json}
                  oninput={(event) => {
                    const target = event.currentTarget as HTMLInputElement | null;
                    replaceRow(index, { value_json: target?.value ?? '' });
                  }}
                />
              </label>

              <button class="ui-button ui-button--subtle" type="button" onclick={() => removeRow(index)}>
                移除
              </button>
            </div>
          {/each}
        {/if}
      </div>

      <div class="canvas__actions">
        <button class="ui-button ui-button--ghost" type="button" onclick={addRow}>新增变量</button>
        <button class="ui-button ui-button--subtle" disabled={!dirty} type="button" onclick={resetDraft}>
          重置
        </button>
        <button class="ui-button ui-button--primary" disabled={saving || !dirty} type="button" onclick={() => void save()}>
          {saving ? '保存中…' : '保存 vars'}
        </button>
      </div>

      <div class="section__title">
        <span>Merged Preview</span>
        <strong>{detail?.merged_vars.length ?? 0}</strong>
      </div>

      {#if !detail || detail.merged_vars.length === 0}
        <p class="empty empty--flush">（当前 profile 没有 merged vars）</p>
      {:else}
        <div class="inspector-table">
          {#each detail.merged_vars as item (item.key)}
            <div class="inspector-row">
              <span class="inspector-row__label">{item.key}</span>
              <div class="inspector-row__value inspector-row__value--stack">
                <span class="inspector-row__value inspector-row__value--mono">{item.value_json}</span>
                <button
                  class="chip"
                  type="button"
                  onclick={() => void onCopyPlaceholder(`{{ ${item.key} }}`, 'Tera 占位符')}
                >
                  复制 {`{{ ${item.key} }}`}
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}

      {#if mergedOnlyRows.length > 0}
        <p class="stack-note">
          继承得到但未在当前 profile 显式声明的变量：
          {mergedOnlyRows.map((item) => item.key).join(', ')}
        </p>
      {/if}
    </div>
  {/if}
</div>
