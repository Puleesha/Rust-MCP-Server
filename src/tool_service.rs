use tokio::time::Duration;

use std::sync::{Arc, atomic::{AtomicUsize, Ordering, AtomicBool}};
use std::thread;
use std::thread::available_parallelism;
use std::path::PathBuf;

use rayon::scope;

use threadpool::ThreadPool;

use crate::repo_analyser::RepoAnalyser;
use crate::request_stats::RequestStats;

pub struct ToolService {
    tasks: ThreadPool
}

impl ToolService {

    pub fn new() -> Self {
        Self {
            tasks: ThreadPool::new(8)
        }
    }
    
    pub fn baseline_tool_process(&self, limit: usize) -> RequestStats {

        let repo_analyser = Arc::new(RepoAnalyser::new(limit));
        let file_paths: Vec<PathBuf> = RepoAnalyser::analyze_repository("app/MockRepository");
        let active_tasks = Arc::new(AtomicUsize::new(file_paths.len()));

        //------------------------------------------------
        // Spawn tasks (unstructured)
        //------------------------------------------------

        for path in file_paths {

            let repo = repo_analyser.clone();
            let active = active_tasks.clone();

            self.tasks.execute(move || {

                if repo.is_limit_reached() {
                    active.fetch_sub(1, Ordering::Relaxed);
                    return;
                }

                repo.analyze_file(path);

                active.fetch_sub(1, Ordering::Relaxed);
            });
        }

        //------------------------------------------------
        // Wait until quota or deadline
        //------------------------------------------------

        while !repo_analyser.is_limit_reached() {
            std::thread::sleep(Duration::from_millis(1));
        }

        // NOTE: Tasks are not awaited to completion (unstructured semantics)

        let unfinished_tasks = active_tasks.load(Ordering::Relaxed);

        eprintln!("Baseline tool called with a imit of = {} TODOs", limit);

        RequestStats {
            todo_count: repo_analyser.get_todo_count(),
            file_count: repo_analyser.get_file_count(),
            unfinished_tasks,
            todo_tasks: repo_analyser.get_todos()
        }
    }

    pub fn structured_tool_process(&self, limit: usize) -> RequestStats {

        let repo_analyser = Arc::new(RepoAnalyser::new(limit));
    
        let file_paths: Vec<PathBuf> = RepoAnalyser::analyze_repository("app/MockRepository/");
    
        let active_tasks = Arc::new(AtomicUsize::new(file_paths.len()));
        let cancelled = Arc::new(AtomicBool::new(false));
        
        //------------------------------------------------
        // Structured rayon scope
        //------------------------------------------------
    
        scope(|task_scope| {
    
            //------------------------------------------------
            // Deadline cancellation task
            //------------------------------------------------
    
            let cancel_flag = cancelled.clone();
            let repo = repo_analyser.clone();
    
            task_scope.spawn(move |_| {
                while !repo.is_limit_reached() {
    
                    if cancel_flag.load(Ordering::Relaxed) {
                        return;
                    }
    
                    thread::sleep(Duration::from_millis(5));
                }
    
                cancel_flag.store(true, Ordering::Relaxed);
            });
    
            //------------------------------------------------
            // Spawn analysis tasks
            //------------------------------------------------
    
            for path in file_paths {
    
                let repo = repo_analyser.clone();
                let active = active_tasks.clone();
                let cancelled = cancelled.clone();
    
                task_scope.spawn(move |_| {
    
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
    
                    repo.analyze_file(path);
    
                    //------------------------------------------------
                    // Check limit condition
                    //------------------------------------------------
    
                    if repo.is_limit_reached() {
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
            todo_tasks: repo_analyser.get_todos()
        }
    }
}