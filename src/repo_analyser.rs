use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::{atomic::{AtomicUsize, Ordering}, Arc};
use walkdir::WalkDir;
use std::sync::Mutex;
use std::mem::take;

pub struct RepoAnalyser {
    file_count: Arc<AtomicUsize>,
    todo_count: Arc<AtomicUsize>,
    todo_tasks: Arc<Mutex<Vec<String>>>
}

impl RepoAnalyser {
    const RESPONSE_LENGTH_LIMIT: usize = 500;

    pub fn new() -> Self {
        Self {
            file_count: Arc::new(AtomicUsize::new(0)),
            todo_count: Arc::new(AtomicUsize::new(0)),
            todo_tasks: Arc::new(Mutex::new(Vec::new()))
        }
    }

    pub fn analyze_repository(path: &str) -> Vec<PathBuf> {
        let root = Path::new(path);
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

        result.sort(); // like Java's .sorted() to return the same set of files for every function call
        result
    }

    pub fn get_file_count(&self) -> usize {
        self.file_count.load(Ordering::SeqCst)
    }

    pub fn get_todo_count(&self) -> usize {
        self.todo_count.load(Ordering::SeqCst)
    }

    pub fn get_todo_tasks(&self) -> Vec<String> {
        let mut guard = self.todo_tasks.lock().unwrap();
        take(&mut *guard)
    }

    pub async fn analyze_file(&self, path: PathBuf, limit: usize, todos: Arc<AtomicUsize>) {
        if let Ok(file) = File::open(&path) {
            self.file_count.fetch_add(1, Ordering::SeqCst);

            let reader = BufReader::new(file);

            for line in reader.lines().flatten() {
                // Stop if limit reached
                if self.todo_count.load(Ordering::SeqCst) >= limit {
                    break;
                }

                if  line.contains("TODO") && 
                    (self.get_response_length().await + line.len() < Self::RESPONSE_LENGTH_LIMIT) 
                {
                    let mut guard = self.todo_tasks.lock().unwrap();
                    guard.push(String::from(line));

                    self.todo_count.fetch_add(1, Ordering::SeqCst);
                    todos.fetch_add(1, Ordering::SeqCst);
                }
            }
        }
    }

    async fn get_response_length(&self) -> usize {
        let guard = self.todo_tasks.lock().unwrap();
        let response_length: usize = guard.iter().map(|s| s.len()).sum();

        response_length
    }
}