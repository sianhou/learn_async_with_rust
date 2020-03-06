use std::{fmt, fs, io::Read, thread};

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
    pub fn read(path: &'static str, cb: impl Fn(Option<String>) + 'static) {
        let work = move || {
            thread::sleep(std::time::Duration::from_secs(2));
            let mut buffer = String::new();
            fs::File::open(&path)
                .unwrap()
                .read_to_string(&mut buffer)
                .unwrap();
            Option::Some(buffer)
        };

        let rt = unsafe { &mut *crate::runtime::RUNTIME };
        rt.thread_pool_event.push((
            Box::new(work),
            ThreadPollTaskKind::FileRead,
            Box::new(cb),
        ));
    }
}
