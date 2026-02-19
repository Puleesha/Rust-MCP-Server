use tokio::task::JoinSet;
use tokio::time::{Instant, Duration, sleep_until};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering}
};
use std::path::PathBuf;

pub async fn baseline_tool_process(limit: usize) -> RequestStats {

    let repo_analyser = Arc::new(RepoAnalyser::new());

    let file_paths: Vec<PathBuf> =
        repo_analyser.analyze_repository("/app/MockRepository/Rust", ".rs");

    let active_tasks = Arc::new(AtomicUsize::new(file_paths.len()));
    let todo_count = Arc::new(AtomicUsize::new(0));

    let deadline = Instant::now() + REQUEST_DEADLINE;

    let mut handles = Vec::with_capacity(file_paths.len());

    //------------------------------------------------
    // Spawn tasks (unstructured)
    //------------------------------------------------

    for path in file_paths {

        let repo = repo_analyser.clone();
        let active = active_tasks.clone();
        let todos = todo_count.clone();

        let handle = tokio::spawn(async move {

            if todos.load(Ordering::Relaxed) >= limit {
                active.fetch_sub(1, Ordering::Relaxed);
                return;
            }

            repo.analyze_file(path, limit, todos.clone()).await;

            active.fetch_sub(1, Ordering::Relaxed);
        });

        handles.push(handle);
    }

    //------------------------------------------------
    // Wait until quota or deadline
    //------------------------------------------------

    while Instant::now() < deadline {

        if todo_count.load(Ordering::Relaxed) >= limit {
            break;
        }

        tokio::task::yield_now().await;
    }

    //------------------------------------------------
    // Best effort cancellation
    //------------------------------------------------

    for handle in &handles {
        handle.abort();
    }

    // NOTE: We DO NOT await them properly (unstructured semantics)

    let unfinished_tasks = active_tasks.load(Ordering::Relaxed);

    RequestStats {
        todo_count: repo_analyser.get_todo_count(),
        file_count: repo_analyser.get_file_count(),
        unfinished_tasks,
    }
}


pub async fn structured_tool_process(limit: usize) -> RequestStats {

    let repo_analyser = Arc::new(RepoAnalyser::new());

    let file_paths: Vec<PathBuf> =
        repo_analyser.analyze_repository("/app/MockRepository/Rust", ".rs");

    let active_tasks = Arc::new(AtomicUsize::new(file_paths.len()));
    let todo_count = Arc::new(AtomicUsize::new(0));

    let deadline = Instant::now() + REQUEST_DEADLINE;

    let mut set = JoinSet::new();

    //------------------------------------------------
    // Structured spawn
    //------------------------------------------------

    for path in file_paths {

        let repo = repo_analyser.clone();
        let active = active_tasks.clone();
        let todos = todo_count.clone();

        set.spawn(async move {

            if todos.load(Ordering::Relaxed) >= limit {
                active.fetch_sub(1, Ordering::Relaxed);
                return;
            }

            repo.analyze_file(path, limit, todos.clone()).await;

            active.fetch_sub(1, Ordering::Relaxed);
        });
    }

    //------------------------------------------------
    // Structured join loop
    //------------------------------------------------

    let mut deadline_sleep = Box::pin(sleep_until(deadline));

    loop {

        tokio::select! {

            _ = &mut deadline_sleep => {
                set.abort_all();
                break;
            }

            Some(_) = set.join_next() => {

                if todo_count.load(Ordering::Relaxed) >= limit {
                    set.abort_all();
                    break;
                }

                if set.is_empty() {
                    break;
                }
            }

            else => break,
        }
    }

    //------------------------------------------------
    // Structured cleanup guarantee
    //------------------------------------------------

    while set.join_next().await.is_some() {}

    let unfinished_tasks = active_tasks.load(Ordering::Relaxed);

    RequestStats {
        todo_count: repo_analyser.get_todo_count(),
        file_count: repo_analyser.get_file_count(),
        unfinished_tasks,
    }
}
