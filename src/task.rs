use std::{fmt};

use crate::runtime::Runtime;

pub struct Task {
    pub(crate) task: Box<dyn Fn() -> Option<String> + Send + 'static>,
    pub(crate) callback_id: usize,
    pub(crate) kind: ThreadPollTaskKind,
}

pub enum ThreadPollTaskKind {
    FileRead,
}

impl fmt::Display for ThreadPollTaskKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ThreadPollTaskKind::FileRead => write!(f, "File read"),
        }
    }
}

pub struct Fs;

impl Fs {
    pub fn read() {
        let mut rt_ptr : *mut Runtime = std::ptr::null_mut();
        unsafe { rt_ptr =crate::runtime:: RUNTIME; }
        println!{"{:p}", rt_ptr};
    }
}