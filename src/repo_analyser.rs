use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex,
};
use std::time::{Duration, Instant};
use walkdir::WalkDir;

pub struct RepoAnalyser {
    file_count: AtomicUsize,
    todo_count: AtomicUsize,
    todo_tasks: Mutex<Vec<String>>,
    mutex: Mutex<u8>,
    deadline: Instant,
    task_limit: usize,
}

impl RepoAnalyser {
    const REQUEST_DEADLINE: Duration = Duration::from_secs(5);
    const RESPONSE_LENGTH_LIMIT: usize = 500;

    pub fn new(limit: usize) -> Self {
        Self {
            file_count: AtomicUsize::new(0),
            todo_count: AtomicUsize::new(0),
            todo_tasks: Mutex::new(Vec::new()),
            mutex: Mutex::new(1),
            deadline: Instant::now() + Self::REQUEST_DEADLINE,
            task_limit: limit,
        }
    }

    pub fn analyze_repository(folder_path: &str) -> Vec<PathBuf> {
        let root = Path::new(folder_path);
        Self::discover_files(root)
    }

    fn discover_files(root_dir: &Path) -> Vec<PathBuf> {
        let mut result = Vec::new();

        for entry in WalkDir::new(root_dir).into_iter().filter_map(Result::ok) {
            let path = entry.path();

            if path.is_file() {
                result.push(path.to_path_buf());
            }
        }

        // same as Java .sorted() to return the same set of files for each request 
        result.sort();
        result
    }

    pub fn analyze_file(&self, path: PathBuf) {
        if let Ok(file) = File::open(&path) {
            self.file_count.fetch_add(1, Ordering::SeqCst);

            let reader = BufReader::new(file);

            for line in reader.lines().flatten() {
                if line.contains("TODO") {
                    self.add_todo(line);
                }
            }
        }
    }

    fn add_todo(&self, line: String) {
        let mut guard = self.todo_tasks.lock().unwrap();

        self.todo_count.fetch_add(1, Ordering::SeqCst);

        // mimic Java: line.replace("//", " ")
        let cleaned = line.replace("//", " ");
        guard.push(cleaned);
    }

    fn get_response_length(&self) -> usize {
        let guard = self.todo_tasks.lock().unwrap();
        guard.iter().map(|s| s.len()).sum()
    }

    pub fn get_file_count(&self) -> usize {
        self.file_count.load(Ordering::SeqCst)
    }

    pub fn get_todo_count(&self) -> usize {
        self.todo_count.load(Ordering::SeqCst)
    }

    pub fn get_todos(&self) -> Vec<String> {
        self.todo_tasks.lock().unwrap().clone()
    }

    pub fn is_limit_reached(&self) -> bool {
        let _lock = self.mutex.lock().unwrap();

        self.todo_count.load(Ordering::SeqCst) >= self.task_limit || 
        (self.get_response_length() >= Self::RESPONSE_LENGTH_LIMIT) || 
        Instant::now() >= self.deadline
    }
}