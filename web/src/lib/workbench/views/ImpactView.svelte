<script lang="ts">
  import SplitView from '$lib/components/SplitView.svelte';
  import type { ImpactAnalysisResponse } from '$lib/types';
  import { truncateMiddle } from '$lib/utils/format';

  type ImpactMode = 'artifact' | 'profile' | 'artifact_profile';

  type Props = {
    impactMode: ImpactMode;
    impact: ImpactAnalysisResponse | null;
    selectedArtifact: string | null;
    selectedProfile: string | null;
    busyImpact: boolean;
    errorMessage: string | null;
    statusLine: string;
    onSetImpactMode: (next: ImpactMode) => void;
    onRefreshImpact: () => Promise<void>;
  };

  let {
    impactMode,
    impact,
    selectedArtifact,
    selectedProfile,
    busyImpact,
    errorMessage,
    statusLine,
    onSetImpactMode,
    onRefreshImpact
  }: Props = $props();
</script>

<aside class="explorer surface" aria-label="资源面板">
  <div class="explorer__head">
    <p class="explorer__eyebrow">IMPACT</p>
    <p class="explorer__hint">选择 subject 后运行分析并查看受影响 targets</p>
  </div>

  <div class="explorer__section">
    <div class="section__title">
      <span>Subject</span>
      <strong>{impact?.affected_targets.length ?? 0}</strong>
    </div>

    <div class="chips">
      <button
        class={['chip', impactMode === 'artifact_profile' ? 'chip--active' : ''].join(' ')}
        onclick={() => onSetImpactMode('artifact_profile')}
        type="button"
      >
        artifact+profile
      </button>
      <button
        class={['chip', impactMode === 'artifact' ? 'chip--active' : ''].join(' ')}
        onclick={() => onSetImpactMode('artifact')}
        type="button"
      >
        artifact
      </button>
      <button
        class={['chip', impactMode === 'profile' ? 'chip--active' : ''].join(' ')}
        onclick={() => onSetImpactMode('profile')}
        type="button"
      >
        profile
      </button>
    </div>

    <div class="list__static">
      <span class="muted">artifact</span>
      <span class="mono">{selectedArtifact ?? '（无）'}</span>
    </div>
    <div class="list__static">
      <span class="muted">profile</span>
      <span class="mono">{selectedProfile ?? '（无）'}</span>
    </div>

    <div class="list__static">
      <span class="muted">issues</span>
      <span class="mono">{impact?.issues.length ?? 0}</span>
    </div>
  </div>
</aside>

<main class="canvas" aria-label="工作区画布">
  <div class="canvas__head">
    <div class="title">
      <strong>Impact</strong>
      <span class="muted">{impact ? `· ${impact.subject_id}` : ''}</span>
    </div>

    <div class="canvas__actions">
      <button
        class="ui-button ui-button--primary"
        disabled={busyImpact}
        type="button"
        onclick={() => void onRefreshImpact()}
      >
        {busyImpact ? '分析中…' : '运行分析'}
      </button>
    </div>
  </div>

  {#if errorMessage}
    <p class="notice notice--error">{errorMessage}</p>
  {/if}
  <p class="status-line" aria-live="polite">{statusLine}</p>

  <div class="split surface">
    <SplitView initialLeftPct={48} minLeftPx={360} minRightPx={360}>
      {#snippet left()}
        <div class="pane">
          <div class="pane__title">Affected Targets</div>
          <div class="pane__body">
            {#if !impact}
              <p class="muted">（尚未运行 impact analysis）</p>
            {:else if impact.affected_targets.length === 0}
              <p class="muted">（没有受影响 targets）</p>
            {:else}
              <ul class="list">
                {#each impact.affected_targets as t (t.id)}
                  <li class="list__static" title={t.target_path}>
                    <span class="mono">{t.id}</span>
                    <span class="muted">{truncateMiddle(t.target_path, 42)}</span>
                  </li>
                {/each}
              </ul>
            {/if}
          </div>
        </div>
      {/snippet}

      {#snippet right()}
        <div class="pane">
          <div class="pane__title">Issues / Link Health</div>
          <div class="pane__body">
            {#if !impact}
              <p class="muted">（暂无数据）</p>
            {:else}
              <div class="meta">
                <div class="meta__row">
                  <span class="meta__label">Artifacts</span>
                  <span class="meta__value mono">{impact.affected_artifacts.length}</span>
                </div>
                <div class="meta__row">
                  <span class="meta__label">Profiles</span>
                  <span class="meta__value mono">{impact.affected_profiles.length}</span>
                </div>
                <div class="meta__row">
                  <span class="meta__label">Issues</span>
                  <span class="meta__value mono">{impact.issues.length}</span>
                </div>
              </div>

              <div class="output">
                <div class="output__title">issues</div>
                {#if impact.issues.length === 0}
                  <p class="muted small">（没有 issues）</p>
                {:else}
                  <ul class="issues">
                    {#each impact.issues as issue (issue.code + issue.subject_id)}
                      <li
                        class={[
                          'issue',
                          issue.severity === 'error' ? 'issue--error' : 'issue--warn'
                        ].join(' ')}
                      >
                        <div class="issue__head">
                          <span class="mono">{issue.scope}:{issue.subject_id}</span>
                          <span class="issue__badge">{issue.severity}</span>
                        </div>
                        <div class="issue__body">{issue.message}</div>
                      </li>
                    {/each}
                  </ul>
                {/if}
              </div>

              <div class="output output--secondary">
                <div class="output__title">link status</div>
                {#if impact.link_status.length === 0}
                  <p class="muted small">（没有 link status）</p>
                {:else}
                  <ul class="list">
                    {#each impact.link_status as item (item.target_path)}
                      <li class="list__static">
                        <span class={['pill', item.ok ? 'pill--ok' : 'pill--warn'].join(' ')}>
                          {item.ok ? 'ok' : 'bad'}
                        </span>
                        <span class="mono">{item.artifact_id}@{item.profile}</span>
                      </li>
                    {/each}
                  </ul>
                {/if}
              </div>
            {/if}
          </div>
        </div>
      {/snippet}
    </SplitView>
  </div>
</main>
