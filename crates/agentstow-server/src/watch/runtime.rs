use notify::{Config, PollWatcher, Watcher};
use notify_debouncer_full::{
    Debouncer, FileIdCache, FileIdMap, RecommendedCache, new_debouncer, new_debouncer_opt,
};

use super::{
    DEBOUNCE_TICK, DEBOUNCE_TIMEOUT, DEFAULT_POLL_INTERVAL, WatchPlan, WatchSpec, WatchStatusHandle,
};

pub(super) fn start_native_debouncer(
    handle: WatchStatusHandle,
    plan: &WatchPlan,
) -> notify::Result<Debouncer<notify::RecommendedWatcher, RecommendedCache>> {
    let plan = plan.clone();
    new_debouncer(DEBOUNCE_TIMEOUT, Some(DEBOUNCE_TICK), move |result| {
        handle.record_debounced(&plan, result);
    })
}

pub(super) fn start_poll_debouncer(
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

pub(super) fn attach_watch_specs<T, C>(
    debouncer: &mut Debouncer<T, C>,
    specs: &[WatchSpec],
) -> notify::Result<()>
where
    T: Watcher,
    C: FileIdCache,
{
    for spec in specs {
        debouncer.watch(&spec.path, spec.recursive_mode)?;
    }

    Ok(())
}
