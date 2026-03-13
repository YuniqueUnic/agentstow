use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use agentstow_core::{ArtifactKind, normalize_for_display};
use agentstow_manifest::{ArtifactDef, DEFAULT_MANIFEST_FILE, Manifest};
use notify::{Config, PollWatcher, RecursiveMode, Watcher};
use notify_debouncer_full::{
    DebounceEventResult, Debouncer, FileIdMap, RecommendedCache, new_debouncer, new_debouncer_opt,
};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(2);
const DEBOUNCE_TIMEOUT: Duration = Duration::from_millis(900);
const DEBOUNCE_TICK: Duration = Duration::from_millis(225);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WatchMode {
    Native,
    Poll,
    Manual,
}

#[derive(Debug, Clone)]
pub(crate) struct WatchStatusSnapshot {
    pub(crate) mode: WatchMode,
    pub(crate) healthy: bool,
    pub(crate) revision: u64,
    pub(crate) poll_interval_ms: Option<u64>,
    pub(crate) last_event: Option<String>,
    pub(crate) last_event_at: Option<String>,
    pub(crate) last_error: Option<String>,
    pub(crate) watch_roots: Vec<String>,
}

impl WatchStatusSnapshot {
    pub(crate) fn manual(watch_roots: Vec<String>, last_error: Option<String>) -> Self {
        Self {
            mode: WatchMode::Manual,
            healthy: false,
            revision: 0,
            poll_interval_ms: None,
            last_event: None,
            last_event_at: None,
            last_error,
            watch_roots,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct WatchStatusHandle {
    inner: Arc<Mutex<WatchState>>,
}

#[derive(Debug)]
struct WatchState {
    snapshot: WatchStatusSnapshot,
    guard: Option<WatcherGuard>,
}

#[derive(Debug)]
#[allow(dead_code)]
enum WatcherGuard {
    Native(Debouncer<notify::RecommendedWatcher, RecommendedCache>),
    Poll(Debouncer<PollWatcher, FileIdMap>),
}

#[derive(Debug, Clone)]
pub(crate) struct WatchPlan {
    specs: Vec<WatchSpec>,
    tracked_files: Vec<PathBuf>,
    tracked_dirs: Vec<PathBuf>,
    ignored_files: Vec<PathBuf>,
    ignored_dirs: Vec<PathBuf>,
    display_roots: Vec<String>,
}

#[derive(Debug, Clone)]
struct WatchSpec {
    path: PathBuf,
    recursive_mode: RecursiveMode,
}

impl WatchStatusHandle {
    pub(crate) fn start(workspace_root: PathBuf) -> Self {
        let plan = WatchPlan::discover(&workspace_root);
        let snapshot = WatchStatusSnapshot::manual(plan.display_roots.clone(), None);
        let handle = Self {
            inner: Arc::new(Mutex::new(WatchState {
                snapshot,
                guard: None,
            })),
        };

        if plan.specs.is_empty() {
            handle.record_error(WatchMode::Manual, None, "没有可监听的 source-of-truth 路径");
            return handle;
        }

        if let Ok(mut debouncer) = start_native_debouncer(handle.clone(), &plan) {
            if let Err(error) = attach_watch_specs(&mut debouncer, &plan.specs) {
                handle.record_error(
                    WatchMode::Native,
                    None,
                    format!("native watcher 挂载失败：{error}"),
                );
            } else {
                handle.install_native(debouncer);
                return handle;
            }
        }

        let poll_interval_ms = DEFAULT_POLL_INTERVAL.as_millis() as u64;
        match start_poll_debouncer(handle.clone(), &plan) {
            Ok(mut debouncer) => {
                if let Err(error) = attach_watch_specs(&mut debouncer, &plan.specs) {
                    handle.record_error(
                        WatchMode::Poll,
                        Some(poll_interval_ms),
                        format!("poll watcher 挂载失败：{error}"),
                    );
                } else {
                    handle.install_poll(debouncer, poll_interval_ms);
                    return handle;
                }
            }
            Err(error) => {
                handle.record_error(
                    WatchMode::Poll,
                    Some(poll_interval_ms),
                    format!("poll watcher 创建失败：{error}"),
                );
            }
        }

        handle.record_error(
            WatchMode::Manual,
            None,
            "native watcher 与 polling watcher 初始化均失败，已退化为 manual refresh",
        );
        handle
    }

    #[allow(dead_code)]
    #[cfg(test)]
    pub(crate) fn from_snapshot(snapshot: WatchStatusSnapshot) -> Self {
        Self {
            inner: Arc::new(Mutex::new(WatchState {
                snapshot,
                guard: None,
            })),
        }
    }

    pub(crate) fn snapshot(&self) -> WatchStatusSnapshot {
        self.inner
            .lock()
            .expect("watch status mutex poisoned")
            .snapshot
            .clone()
    }

    fn install_native(&self, debouncer: Debouncer<notify::RecommendedWatcher, RecommendedCache>) {
        let mut state = self.inner.lock().expect("watch status mutex poisoned");
        state.snapshot.mode = WatchMode::Native;
        state.snapshot.healthy = true;
        state.snapshot.poll_interval_ms = None;
        state.snapshot.last_error = None;
        state.guard = Some(WatcherGuard::Native(debouncer));
    }

    fn install_poll(&self, debouncer: Debouncer<PollWatcher, FileIdMap>, poll_interval_ms: u64) {
        let mut state = self.inner.lock().expect("watch status mutex poisoned");
        state.snapshot.mode = WatchMode::Poll;
        state.snapshot.healthy = true;
        state.snapshot.poll_interval_ms = Some(poll_interval_ms);
        state.snapshot.last_error = None;
        state.guard = Some(WatcherGuard::Poll(debouncer));
    }

    fn record_debounced(&self, plan: &WatchPlan, result: DebounceEventResult) {
        match result {
            Ok(events) => {
                let summary = summarize_events(plan, &events);
                if summary.is_none() {
                    return;
                }

                let mut state = self.inner.lock().expect("watch status mutex poisoned");
                state.snapshot.healthy = true;
                state.snapshot.revision = state.snapshot.revision.saturating_add(1);
                state.snapshot.last_event = summary;
                state.snapshot.last_event_at = Some(now_rfc3339());
                state.snapshot.last_error = None;
            }
            Err(errors) => {
                let mut state = self.inner.lock().expect("watch status mutex poisoned");
                state.snapshot.healthy = false;
                state.snapshot.last_error = Some(
                    errors
                        .into_iter()
                        .map(|error| error.to_string())
                        .collect::<Vec<_>>()
                        .join(" | "),
                );
                state.snapshot.last_event_at = Some(now_rfc3339());
            }
        }
    }

    fn record_error(&self, mode: WatchMode, poll_interval_ms: Option<u64>, error: impl ToString) {
        let mut state = self.inner.lock().expect("watch status mutex poisoned");
        state.snapshot.mode = mode;
        state.snapshot.healthy = false;
        state.snapshot.poll_interval_ms = poll_interval_ms;
        state.snapshot.last_error = Some(error.to_string());
        state.snapshot.last_event_at = Some(now_rfc3339());
    }
}

impl WatchPlan {
    pub(crate) fn discover(workspace_root: &Path) -> Self {
        let manifest_path = workspace_root.join(DEFAULT_MANIFEST_FILE);
        let mut tracked_files = vec![manifest_path];
        let mut tracked_dirs = Vec::new();
        let mut ignored_files = Vec::new();
        let mut ignored_dirs = Vec::new();
        let mut specs = Vec::new();

        add_watch_spec(
            &mut specs,
            workspace_root.to_path_buf(),
            RecursiveMode::NonRecursive,
        );

        if let Ok(manifest) = Manifest::load_from_dir(workspace_root) {
            for artifact in manifest.artifacts.values() {
                add_artifact_sources(
                    &mut tracked_files,
                    &mut tracked_dirs,
                    &mut specs,
                    workspace_root,
                    artifact,
                );
            }

            add_overlapping_target_ignores(
                &manifest,
                workspace_root,
                &tracked_files,
                &tracked_dirs,
                &mut ignored_files,
                &mut ignored_dirs,
            );
        }

        collapse_watch_specs(&mut specs);
        dedupe_paths(&mut tracked_files);
        dedupe_paths(&mut tracked_dirs);
        dedupe_paths(&mut ignored_files);
        dedupe_paths(&mut ignored_dirs);
        let display_roots = specs
            .iter()
            .map(|spec| normalize_for_display(&spec.path))
            .collect();

        Self {
            specs,
            tracked_files,
            tracked_dirs,
            ignored_files,
            ignored_dirs,
            display_roots,
        }
    }

    #[cfg(test)]
    pub(crate) fn for_test(
        tracked_files: Vec<PathBuf>,
        tracked_dirs: Vec<PathBuf>,
        ignored_files: Vec<PathBuf>,
        ignored_dirs: Vec<PathBuf>,
    ) -> Self {
        Self {
            specs: Vec::new(),
            tracked_files,
            tracked_dirs,
            ignored_files,
            ignored_dirs,
            display_roots: Vec::new(),
        }
    }

    pub(crate) fn matches_path(&self, path: &Path) -> bool {
        if self.ignores_path(path) {
            return false;
        }

        self.tracked_files.iter().any(|tracked| tracked == path)
            || self
                .tracked_dirs
                .iter()
                .any(|tracked| path == tracked || path.starts_with(tracked))
    }

    fn ignores_path(&self, path: &Path) -> bool {
        self.ignored_files.iter().any(|ignored| ignored == path)
            || self
                .ignored_dirs
                .iter()
                .any(|ignored| path == ignored || path.starts_with(ignored))
    }
}

fn start_native_debouncer(
    handle: WatchStatusHandle,
    plan: &WatchPlan,
) -> notify::Result<Debouncer<notify::RecommendedWatcher, RecommendedCache>> {
    let plan = plan.clone();
    new_debouncer(DEBOUNCE_TIMEOUT, Some(DEBOUNCE_TICK), move |result| {
        handle.record_debounced(&plan, result);
    })
}

fn start_poll_debouncer(
    handle: WatchStatusHandle,
    plan: &WatchPlan,
) -> notify::Result<Debouncer<PollWatcher, FileIdMap>> {
    let plan = plan.clone();
    let config = Config::default().with_poll_interval(DEFAULT_POLL_INTERVAL);
    new_debouncer_opt::<_, PollWatcher, FileIdMap>(
        DEBOUNCE_TIMEOUT,
        Some(DEBOUNCE_TICK),
        move |result| {
            handle.record_debounced(&plan, result);
        },
        FileIdMap::new(),
        config,
    )
}

fn add_artifact_sources(
    tracked_files: &mut Vec<PathBuf>,
    tracked_dirs: &mut Vec<PathBuf>,
    specs: &mut Vec<WatchSpec>,
    workspace_root: &Path,
    artifact: &ArtifactDef,
) {
    let source_path = artifact.source_path(workspace_root);

    match artifact.kind {
        ArtifactKind::Dir => {
            tracked_dirs.push(source_path.clone());
            let existing_ancestor = nearest_existing_ancestor(&source_path, workspace_root);
            add_watch_spec(specs, existing_ancestor, RecursiveMode::Recursive);
        }
        ArtifactKind::File => {
            tracked_files.push(source_path.clone());
            let parent = source_path.parent().unwrap_or(workspace_root);
            let existing_ancestor = nearest_existing_ancestor(parent, workspace_root);
            let recursive_mode = if existing_ancestor == parent {
                RecursiveMode::NonRecursive
            } else {
                RecursiveMode::Recursive
            };
            add_watch_spec(specs, existing_ancestor, recursive_mode);
        }
    }
}

fn add_overlapping_target_ignores(
    manifest: &Manifest,
    workspace_root: &Path,
    tracked_files: &[PathBuf],
    tracked_dirs: &[PathBuf],
    ignored_files: &mut Vec<PathBuf>,
    ignored_dirs: &mut Vec<PathBuf>,
) {
    for target in manifest.targets.values() {
        let Some(artifact) = manifest.artifacts.get(&target.artifact) else {
            continue;
        };

        let target_path = target.absolute_target_path(workspace_root);
        match artifact.kind {
            ArtifactKind::Dir => {
                if tracked_dirs
                    .iter()
                    .any(|tracked| target_path == *tracked || target_path.starts_with(tracked))
                {
                    ignored_dirs.push(target_path);
                }
            }
            ArtifactKind::File => {
                if tracked_files.iter().any(|tracked| tracked == &target_path)
                    || tracked_dirs
                        .iter()
                        .any(|tracked| target_path.starts_with(tracked))
                {
                    ignored_files.push(target_path);
                }
            }
        }
    }
}

fn add_watch_spec(specs: &mut Vec<WatchSpec>, path: PathBuf, recursive_mode: RecursiveMode) {
    if let Some(existing) = specs.iter_mut().find(|spec| spec.path == path) {
        if matches!(recursive_mode, RecursiveMode::Recursive) {
            existing.recursive_mode = RecursiveMode::Recursive;
        }
        return;
    }

    specs.push(WatchSpec {
        path,
        recursive_mode,
    });
}

fn collapse_watch_specs(specs: &mut Vec<WatchSpec>) {
    specs.sort_by(|left, right| left.path.cmp(&right.path));
    let mut collapsed = Vec::new();

    for spec in specs.drain(..) {
        if collapsed.iter().any(|existing: &WatchSpec| {
            matches!(existing.recursive_mode, RecursiveMode::Recursive)
                && (spec.path == existing.path || spec.path.starts_with(&existing.path))
        }) {
            continue;
        }

        if matches!(spec.recursive_mode, RecursiveMode::Recursive) {
            collapsed.retain(|existing: &WatchSpec| {
                !(matches!(existing.recursive_mode, RecursiveMode::Recursive)
                    && (existing.path == spec.path || existing.path.starts_with(&spec.path)))
            });
        }

        collapsed.push(spec);
    }

    *specs = collapsed;
}

fn dedupe_paths(paths: &mut Vec<PathBuf>) {
    paths.sort();
    paths.dedup();
}

fn nearest_existing_ancestor(path: &Path, fallback: &Path) -> PathBuf {
    let mut current = Some(path);
    while let Some(candidate) = current {
        if candidate.exists() {
            return candidate.to_path_buf();
        }
        current = candidate.parent();
    }

    fallback.to_path_buf()
}

fn attach_watch_specs<T, C>(
    debouncer: &mut Debouncer<T, C>,
    specs: &[WatchSpec],
) -> notify::Result<()>
where
    T: Watcher,
    C: notify_debouncer_full::FileIdCache,
{
    for spec in specs {
        debouncer.watch(&spec.path, spec.recursive_mode)?;
    }

    Ok(())
}

pub(crate) fn summarize_events(
    plan: &WatchPlan,
    events: &[notify_debouncer_full::DebouncedEvent],
) -> Option<String> {
    let mut count = 0usize;
    let mut last_kind = None;
    let mut paths = BTreeSet::new();

    for event in events {
        if event.kind.is_access() {
            continue;
        }

        let matched_paths: Vec<_> = event
            .paths
            .iter()
            .filter(|path| plan.matches_path(path))
            .map(|path| normalize_for_display(path))
            .collect();

        if matched_paths.is_empty() {
            continue;
        }

        count = count.saturating_add(1);
        last_kind = Some(format!("{:?}", event.kind));
        for path in matched_paths {
            paths.insert(path);
        }
    }

    if count == 0 {
        return None;
    }

    let kind = last_kind.unwrap_or_else(|| "Change".to_string());
    let path_summary = match paths.len() {
        0 => "source-of-truth".to_string(),
        1 => paths.iter().next().cloned().unwrap_or_default(),
        total => format!("{total} 个源路径"),
    };

    Some(format!("{} 条事件 · {} · {}", count, kind, path_summary))
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| OffsetDateTime::now_utc().unix_timestamp().to_string())
}
