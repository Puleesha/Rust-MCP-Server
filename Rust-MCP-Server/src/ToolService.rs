use tokio::task::JoinSet;
use tokio::time::{Instant, Duration, sleep_until};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering}
};
use std::path::PathBuf;

pub async fn structured_tool_process(limit: usize) -> RequestStats {

    let repo_analyser = Arc::new(RepoAnalyser::new());

    let file_paths: Vec<PathBuf> =
        repo_analyser.analyze_repository("/app/MockRepository/Rust", ".rs");

    let active_tasks = Arc::new(AtomicUsize::new(file_paths.len()));
    let todo_count = Arc::new(AtomicUsize::new(0));

    let deadline = Instant::now() + REQUEST_DEADLINE;

    let mut set = JoinSet::new();

    //------------------------------------------------
    // Spawn tasks (structured scope)
    //------------------------------------------------

    for path in file_paths {

        let repo = repo_analyser.clone();
        let active = active_tasks.clone();
        let todos = todo_count.clone();

        set.spawn(async move {

            // cooperative cancellation check
            if todos.load(Ordering::Relaxed) >= limit {
                active.fetch_sub(1, Ordering::Relaxed);
                return;
            }

            repo.analyze_file(path, limit, todos.clone()).await;

            active.fetch_sub(1, Ordering::Relaxed);
        });
    }

    //------------------------------------------------
    // Join loop with quota + deadline enforcement
    //------------------------------------------------

    let mut deadline_sleep = sleep_until(deadline);

    loop {

        tokio::select! {

            //------------------------------------------------
            // Deadline reached
            //------------------------------------------------
            _ = &mut deadline_sleep => {

                set.abort_all();
                break;
            }

            //------------------------------------------------
            // Task completed
            //------------------------------------------------
            Some(result) = set.join_next() => {

                // ignore task result, semantics handled via counters

                if todo_count.load(Ordering::Relaxed) >= limit {

                    set.abort_all();
                    break;
                }

                if set.is_empty() {
                    break;
                }
            }

            //------------------------------------------------
            // No more tasks
            //------------------------------------------------
            else => {
                break;
            }
        }
    }

    //------------------------------------------------
    // Drain cancelled tasks (structured cleanup)
    //------------------------------------------------

    while set.join_next().await.is_some() {}

    let unfinished_tasks = active_tasks.load(Ordering::Relaxed);

    RequestStats {
        todo_count: repo_analyser.get_todo_count(),
        file_count: repo_analyser.get_file_count(),
        unfinished_tasks,
    }
}
