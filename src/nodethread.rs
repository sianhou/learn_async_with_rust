use std::{
    thread,
    sync::mpsc::Sender};

use crate::task;

#[derive(Debug)]
pub struct NodeThread {
    pub(crate) handle: thread::JoinHandle<()>,
    pub(crate) sender: Sender<task::Task>,
}