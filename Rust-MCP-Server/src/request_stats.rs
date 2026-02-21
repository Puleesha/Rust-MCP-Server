#[derive(Debug, Clone)]
pub struct RequestStats {
    pub todo_count: usize,
    pub file_count: u32,
    pub unfinished_tasks: u32,
}