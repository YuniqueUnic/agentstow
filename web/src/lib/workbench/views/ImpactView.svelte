<script lang="ts">
  import { Tabs } from 'bits-ui';

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
    onOpenTarget: (id: string) => void;
    onOpenArtifact: (id: string) => void;
    onOpenProfile: (id: string) => void;
  };

  let {
    impactMode,
    impact,
    selectedArtifact,
    selectedProfile,
    busyImpact,
    onSetImpactMode,
    onRefreshImpact,
    onOpenTarget,
    onOpenArtifact,
    onOpenProfile
  }: Props = $props();

  let panelTab = $state<'issues' | 'link_status' | 'summary'>('issues');
</script>

<SplitView autoSaveId="workbench:view:impact" initialLeftPct={22} minLeftPx={256} minRightPx={760}>
  {#snippet left()}
    <aside class="explorer surface" aria-label="资源面板">
      <div class="explorer__head">
        <p class="explorer__eyebrow">IMPACT</p>
        <p class="explorer__hint">当前展示的是基于 artifact/profile 选择收集出的影响结果摘要，而不是独立的依赖图分析引擎。</p>
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

        <div class="subject-summary">
          <div class="summary-row">
            <span class="summary-row__label">Artifact</span>
            <span class="summary-row__value mono">{selectedArtifact ?? '（无）'}</span>
          </div>
          <div class="summary-row">
            <span class="summary-row__label">Profile</span>
            <span class="summary-row__value mono">{selectedProfile ?? '（无）'}</span>
          </div>
          <div class="summary-row">
            <span class="summary-row__label">Subject</span>
            <span class="summary-row__value mono">{impact?.subject_id ?? '（尚未分析）'}</span>
          </div>
        </div>
      </div>
    </aside>
  {/snippet}

  {#snippet right()}
    <main class="canvas" aria-label="工作区画布">
      <div class="canvas__head">
        <div class="title">
          <strong>Impact Summary</strong>
          <span class="muted">{impact ? `· ${impact.subject_id}` : '· filtered workspace view'}</span>
        </div>

        <div class="canvas__actions">
          <button
            class="ui-button ui-button--primary"
            disabled={busyImpact}
            type="button"
            onclick={() => void onRefreshImpact()}
          >
            {busyImpact ? '刷新中…' : '刷新结果'}
          </button>
        </div>
      </div>
      <div class="canvas__body">
        <SplitView
          autoSaveId="workbench:impact:stack"
          direction="vertical"
          initialLeftPct={58}
          minLeftPx={260}
          minRightPx={180}
        >
          {#snippet left()}
            <SplitView autoSaveId="workbench:impact:document" initialLeftPct={68} minLeftPx={400} minRightPx={260}>
              {#snippet left()}
                <section class="region" aria-label="受影响 targets">
                  <div class="region__header">
                    <span>Affected Targets</span>
                    <span class="mono">{impact?.affected_targets.length ?? 0} results</span>
                  </div>

                  <div class="panel__body panel__body--flush">
                    {#if !impact}
                      <p class="empty empty--flush">（尚未加载 impact summary）</p>
                    {:else if impact.affected_targets.length === 0}
                      <p class="empty empty--flush">（没有受影响 targets）</p>
                    {:else}
                      <ul class="result-list">
                        {#each impact.affected_targets as target (target.id)}
                          <li class="result-row">
                            <button
                              class="result-row__button"
                              onclick={() => onOpenTarget(target.id)}
                              title={target.target_path}
                              type="button"
                            >
                              <span class="pill pill--neutral">{target.method}</span>
                              <span class="result-row__title">{target.id}</span>
                              <span class="result-row__detail">{truncateMiddle(target.target_path, 96)}</span>
                            </button>
                          </li>
                        {/each}
                      </ul>
                    {/if}
                  </div>
                </section>
              {/snippet}

              {#snippet right()}
                <section class="region secondary-sidebar" aria-label="Impact objects sidebar">
                  <div class="region__header">
                    <span>Impact Objects</span>
                    <span class="mono">
                      {(impact?.affected_artifacts.length ?? 0) + (impact?.affected_profiles.length ?? 0)}
                    </span>
                  </div>

                  <div class="panel__body panel__body--flush">
                    {#if !impact}
                      <p class="empty empty--flush">（加载结果后可查看 artifacts / profiles 导航）</p>
                    {:else}
                      <div class="inspector-section">
                        <div class="section__title">
                          <span>Artifacts</span>
                          <strong>{impact.affected_artifacts.length}</strong>
                        </div>

                        {#if impact.affected_artifacts.length === 0}
                          <p class="empty empty--flush">（没有受影响 artifacts）</p>
                        {:else}
                          <div class="token-action-list">
                            {#each impact.affected_artifacts as artifact (artifact.id)}
                              <button
                                class="token token--interactive token--object"
                                onclick={() => onOpenArtifact(artifact.id)}
                                type="button"
                              >
                                <span>{artifact.id}</span>
                                <span class="token__meta">
                                  {artifact.kind} · {artifact.target_ids.length} targets
                                </span>
                              </button>
                            {/each}
                          </div>
                        {/if}
                      </div>

                      <div class="inspector-section">
                        <div class="section__title">
                          <span>Profiles</span>
                          <strong>{impact.affected_profiles.length}</strong>
                        </div>

                        {#if impact.affected_profiles.length === 0}
                          <p class="empty empty--flush">（没有受影响 profiles）</p>
                        {:else}
                          <div class="token-action-list">
                            {#each impact.affected_profiles as profile (profile.id)}
                              <button
                                class="token token--interactive token--object"
                                onclick={() => onOpenProfile(profile.id)}
                                type="button"
                              >
                                <span>{profile.id}</span>
                                <span class="token__meta">
                                  {profile.artifact_ids.length} artifacts · {profile.target_ids.length} targets
                                </span>
                              </button>
                            {/each}
                          </div>
                        {/if}
                      </div>
                    {/if}
                  </div>
                </section>
              {/snippet}
            </SplitView>
          {/snippet}

          {#snippet right()}
            <section class="panel bottom-panel" aria-label="Impact 底部面板">
              <Tabs.Root value={panelTab} onValueChange={(next) => (panelTab = next as typeof panelTab)}>
                <div class="region__header">
                  <Tabs.List class="tabs" aria-label="Impact panel tabs">
                    <Tabs.Trigger class="tab" value="issues">Issues</Tabs.Trigger>
                    <Tabs.Trigger class="tab" value="link_status">Link Health</Tabs.Trigger>
                    <Tabs.Trigger class="tab" value="summary">Summary</Tabs.Trigger>
                  </Tabs.List>
                  <span class="mono">{impact?.subject_id ?? 'idle'}</span>
                </div>

                <Tabs.Content class="panel__body" value="issues">
                  {#if !impact}
                    <p class="empty empty--flush">（暂无问题数据）</p>
                  {:else if impact.issues.length === 0}
                    <p class="empty empty--flush">（没有 issues）</p>
                  {:else}
                    <ul class="issues">
                      {#each impact.issues as issue (issue.code + issue.subject_id)}
                        <li class={['issue', issue.severity === 'error' ? 'issue--error' : 'issue--warn'].join(' ')}>
                          <div class="issue__head">
                            <span class="mono">{issue.scope}:{issue.subject_id}</span>
                            <span class="issue__badge">{issue.severity}</span>
                          </div>
                          <div class="issue__body">{issue.message}</div>
                        </li>
                      {/each}
                    </ul>
                  {/if}
                </Tabs.Content>

                <Tabs.Content class="panel__body" value="link_status">
                  {#if !impact}
                    <p class="empty empty--flush">（暂无 link 健康度数据）</p>
                  {:else if impact.link_status.length === 0}
                    <p class="empty empty--flush">（没有 link status）</p>
                  {:else}
                    <ul class="result-list">
                      {#each impact.link_status as item (item.target_path)}
                        <li class="result-row result-row--triple">
                          <span class={['pill', item.ok ? 'pill--ok' : 'pill--warn'].join(' ')}>
                            {item.ok ? 'ok' : 'bad'}
                          </span>
                          <div class="result-row__main">
                            <span class="result-row__title">{item.artifact_id}@{item.profile}</span>
                            <span class="result-row__detail">{truncateMiddle(item.target_path, 84)}</span>
                          </div>
                          <span class="mono muted">{item.message}</span>
                        </li>
                      {/each}
                    </ul>
                  {/if}
                </Tabs.Content>

                <Tabs.Content class="panel__body" value="summary">
                  {#if !impact}
                    <p class="empty empty--flush">（暂无 summary）</p>
                  {:else}
                    <div class="inspector-table">
                      <div class="inspector-row">
                        <span class="inspector-row__label">Subject</span>
                        <span class="inspector-row__value inspector-row__value--mono">{impact.subject_id}</span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">Artifacts</span>
                        <span class="inspector-row__value inspector-row__value--mono">
                          {impact.affected_artifacts.length}
                        </span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">Profiles</span>
                        <span class="inspector-row__value inspector-row__value--mono">
                          {impact.affected_profiles.length}
                        </span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">Targets</span>
                        <span class="inspector-row__value inspector-row__value--mono">
                          {impact.affected_targets.length}
                        </span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">Issues</span>
                        <span class="inspector-row__value inspector-row__value--mono">{impact.issues.length}</span>
                      </div>
                    </div>
                  {/if}
                </Tabs.Content>
              </Tabs.Root>
            </section>
          {/snippet}
        </SplitView>
      </div>
    </main>
  {/snippet}
</SplitView>
