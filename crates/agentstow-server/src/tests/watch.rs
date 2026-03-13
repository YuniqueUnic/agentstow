use std::time::Instant;

use agentstow_core::normalize_for_display;
use assert_fs::prelude::*;
use notify::event::{AccessKind, AccessMode, DataChange, ModifyKind};
use notify::{Event, EventKind};
use notify_debouncer_full::DebouncedEvent;

use crate::{WatchPlan, summarize_events};

#[test]
fn watch_plan_should_ignore_nested_target_paths() {
    let source_root = std::path::PathBuf::from("/workspace/artifacts/source");
    let source_path = source_root.join("rule.md");
    let generated_root = source_root.join(".generated");
    let generated_path = generated_root.join("rule.md");
    let plan = WatchPlan::for_test(vec![], vec![source_root], vec![], vec![generated_root]);

    assert!(plan.matches_path(&source_path));
    assert!(!plan.matches_path(&generated_path));
}

#[test]
fn summarize_events_should_ignore_access_only_changes() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("artifacts").create_dir_all().unwrap();
    temp.child("artifacts/config.toml")
        .write_str("name = 'agentstow'\n")
        .unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[artifacts.config]
kind = "file"
source = "artifacts/config.toml"
template = false
"#,
        )
        .unwrap();

    let plan = WatchPlan::discover(temp.path());
    let source_path = temp.child("artifacts/config.toml").path().to_path_buf();

    let access_event = DebouncedEvent::new(
        Event {
            kind: EventKind::Access(AccessKind::Open(AccessMode::Read)),
            paths: vec![source_path.clone()],
            attrs: Default::default(),
        },
        Instant::now(),
    );
    assert_eq!(summarize_events(&plan, &[access_event]), None);

    let modify_event = DebouncedEvent::new(
        Event {
            kind: EventKind::Modify(ModifyKind::Data(DataChange::Content)),
            paths: vec![source_path.clone()],
            attrs: Default::default(),
        },
        Instant::now(),
    );
    let summary = summarize_events(&plan, &[modify_event]).unwrap();
    assert!(summary.contains("1 条事件"));
    assert!(summary.contains(&normalize_for_display(&source_path)));
}
