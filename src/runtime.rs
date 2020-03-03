use crate::node_thread::NodeThread;

pub struct Runtime {
    thread_pool: Vec<NodeThread>,
}

impl Runtime {
    pub fn new() -> Self {
        Runtime {
            thread_pool:  Vec::with_capacity(4),
        }
    }
}