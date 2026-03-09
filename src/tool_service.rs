use tokio::time::{Instant, Duration};

use std::sync::{Arc, atomic::{AtomicUsize, Ordering, AtomicBool}};
use std::thread;
use std::path::PathBuf;

use crate::repo_analyser::RepoAnalyser;
use crate::request_stats::RequestStats;

pub struct ToolService;

impl ToolService {

    const REQUEST_DEADLINE: Duration = Duration::from_secs(5);

    pub fn new() -> Self {Self {}}
    
    pub async fn baseline_tool_process(&self, limit: usize) -> RequestStats {

        let repo_analyser = Arc::new(RepoAnalyser::new());

        let file_paths: Vec<PathBuf> = RepoAnalyser::analyze_repository("app/MockRepository");

        let active_tasks = Arc::new(AtomicUsize::new(file_paths.len()));
        let todo_count = Arc::new(AtomicUsize::new(0));

        let deadline: Instant = Instant::now() + Self::REQUEST_DEADLINE;

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

        // //------------------------------------------------
        // // Best effort cancellation
        // //------------------------------------------------

        for handle in &handles {
            handle.abort();
        }

        // NOTE: We DO NOT await them properly (unstructured semantics)

        let unfinished_tasks = active_tasks.load(Ordering::Relaxed);

        eprintln!("Baseline tool called with a imit of = {} TODOs", limit);

        RequestStats {
            todo_count: repo_analyser.get_todo_count(),
            file_count: repo_analyser.get_file_count(),
            unfinished_tasks,
            todo_tasks: repo_analyser.get_todo_tasks()
        }
    }

    pub fn structured_tool_process(&self, limit: usize) -> RequestStats {

        let repo_analyser = Arc::new(RepoAnalyser::new());
    
        let file_paths: Vec<PathBuf> = RepoAnalyser::analyze_repository("app/MockRepository/");
    
        let active_tasks = Arc::new(AtomicUsize::new(file_paths.len()));
        let todo_count = Arc::new(AtomicUsize::new(0));
        let cancelled = Arc::new(AtomicBool::new(false));
    
        let deadline = Instant::now() + Self::REQUEST_DEADLINE;
    
        //------------------------------------------------
        // Structured thread scope
        //------------------------------------------------
    
        thread::scope(|scope| {
    
            //------------------------------------------------
            // Deadline cancellation thread
            //------------------------------------------------
    
            let cancel_flag = cancelled.clone();
    
            scope.spawn(move || {
                while Instant::now() < deadline {
                    if cancel_flag.load(Ordering::Relaxed) {
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
    
                cancel_flag.store(true, Ordering::Relaxed);
            });
    
            //------------------------------------------------
            // Spawn analysis threads
            //------------------------------------------------
    
            for path in file_paths {
    
                let repo = repo_analyser.clone();
                let active = active_tasks.clone();
                let todos = todo_count.clone();
                let cancelled = cancelled.clone();
    
                scope.spawn(move || {
    
                    //------------------------------------------------
                    // Early cancellation check
                    //------------------------------------------------
    
                    if cancelled.load(Ordering::Relaxed) {
                        active.fetch_sub(1, Ordering::Relaxed);
                        return;
                    }
    
                    //------------------------------------------------
                    // Perform file analysis
                    //------------------------------------------------
    
                    repo.analyze_file(path, limit, todos.clone());
    
                    //------------------------------------------------
                    // Check limit condition
                    //------------------------------------------------
    
                    if todos.load(Ordering::Relaxed) >= limit {
                        cancelled.store(true, Ordering::Relaxed);
                    }
    
                    //------------------------------------------------
                    // Mark task completion
                    //------------------------------------------------
    
                    active.fetch_sub(1, Ordering::Relaxed);
                });
            }
    
        });
    
        //------------------------------------------------
        // Final statistics
        //------------------------------------------------
    
        let unfinished_tasks = active_tasks.load(Ordering::Relaxed);
    
        eprintln!("Structured tool called with a limit of = {} TODOs", limit);
    
        RequestStats {
            todo_count: repo_analyser.get_todo_count(),
            file_count: repo_analyser.get_file_count(),
            unfinished_tasks,
            todo_tasks: repo_analyser.get_todo_tasks()
        }
    }
}