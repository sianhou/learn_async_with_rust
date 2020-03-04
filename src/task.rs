use std::{fmt};

pub struct Task {
    task: Box<dyn Fn() -> Option<String> + Send + 'static>,
    pub kind: ThreadPollTaskKind,
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