use std::{sync::mpsc::Sender, thread};

use crate::task;

#[derive(Debug)]
pub struct NodeThread {
    pub(crate) handle: thread::JoinHandle<()>,
    pub(crate) sender: Sender<task::Task>,
}
