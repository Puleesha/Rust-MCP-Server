use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::AtomicUsize
};

// TODO: These methods

pub struct RepoAnalyser {}

impl RepoAnalyser {
    pub fn new() -> Self {Self{}}

    pub fn analyze_repository(path: &str, types: &str) -> Vec<PathBuf> {Vec::new()}

    pub fn get_file_count(&self) -> usize {8} 

    pub fn get_todo_count(&self) -> usize {8} 

    pub async fn analyze_file(path: PathBuf, limit: usize, todos: Arc<AtomicUsize>) {}
}