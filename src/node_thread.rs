use std::thread;

#[derive(Debug)]
pub struct NodeThread {
    pub(crate) handle: thread::JoinHandle<()>,
}