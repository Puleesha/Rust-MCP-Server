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
    pub fn new() -> Self {
        Self {
            file_count: Arc::new(AtomicUsize::new(0)),
            todo_count: Arc::new(AtomicUsize::new(0)),
            todo_tasks: Arc::new(Mutex::new(Vec::new()))
        }
    }

    pub fn analyze_repository(path: &str, types: &str) -> Vec<PathBuf> {
        let root = Path::new(path);
        Self::discover_files(root, types)
    }

    fn discover_files(root_dir: &Path, extension: &str) -> Vec<PathBuf> {
        let mut result = Vec::new();
        let extension = extension.to_lowercase();

        for entry in WalkDir::new(root_dir).into_iter().filter_map(Result::ok) {
            let path = entry.path();

            if path.is_file() && Self::has_allowed_extension(path, &extension) {
                result.push(path.to_path_buf());
            }
        }

        result.sort(); // same as Java's .sorted()
        result
    }

    fn has_allowed_extension(path: &Path, extension: &str) -> bool {
        match path.extension() {
            Some(ext) => ext.to_string_lossy().to_lowercase() == extension,
            None => false,
        }
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

                if line.contains("TODO") {
                    let mut guard = self.todo_tasks.lock().unwrap();
                    guard.push(String::from(line));

                    self.todo_count.fetch_add(1, Ordering::SeqCst);
                    todos.fetch_add(1, Ordering::SeqCst);
                }
            }
        }
    }
}