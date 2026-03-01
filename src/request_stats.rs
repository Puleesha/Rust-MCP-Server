#[derive(Debug, Clone)]
pub struct RequestStats {
    pub todo_count: usize,
    pub file_count: usize,
    pub unfinished_tasks: usize,
    pub todo_tasks: Vec<String>
}