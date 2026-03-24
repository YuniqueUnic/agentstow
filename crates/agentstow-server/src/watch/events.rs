use std::collections::BTreeSet;

use agentstow_core::normalize_for_display;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::{MAX_RECENT_EVENTS, WatchPlan, WatchTraceEvent};

pub(super) fn push_recent_event(events: &mut Vec<WatchTraceEvent>, event: WatchTraceEvent) {
    events.insert(0, event);
    if events.len() > MAX_RECENT_EVENTS {
        events.truncate(MAX_RECENT_EVENTS);
    }
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

pub(super) fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| OffsetDateTime::now_utc().unix_timestamp().to_string())
}
