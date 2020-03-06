use std::{fmt, fs, io::Read, thread};

use crate::ioresult::IOResult;

pub struct Task {
    pub(crate) task: Box<dyn Fn() -> IOResult + Send + 'static>,
    pub(crate) callback_id: usize,
    pub(crate) kind: ThreadPollTaskKind,
}

pub enum ThreadPollTaskKind {
    FileRead,
    CalFibonacchi,
}

impl fmt::Display for ThreadPollTaskKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ThreadPollTaskKind::FileRead => write!(f, "FileRead"),
            ThreadPollTaskKind::CalFibonacchi => write!(f, "CalFibonacchi"),
        }
    }
}

pub struct Fs;
impl Fs {
    pub fn read(path: &'static str, cb: impl Fn(IOResult) + 'static) {
        let work = move || {
            thread::sleep(std::time::Duration::from_secs(2));
            let mut buffer = String::new();
            fs::File::open(&path)
                .unwrap()
                .read_to_string(&mut buffer)
                .unwrap();
            IOResult::String(buffer)
        };

        let rt = unsafe { &mut *crate::runtime::RUNTIME };
        rt.thread_pool_event
            .push((Box::new(work), ThreadPollTaskKind::FileRead, Box::new(cb)));
    }
}

pub struct Fibonacchi;
impl Fibonacchi {
    pub fn cal(n: usize, cb: impl Fn(IOResult) + 'static) {
        let work = move || {
            fn fibonacchi(n: usize) -> usize {
                match n {
                    0 => 0,
                    1 => 1,
                    _ => fibonacchi(n - 1) + fibonacchi(n - 2),
                }
            };

            IOResult::Int(fibonacchi(n))
        };

        let rt = unsafe { &mut *crate::runtime::RUNTIME };
        rt.thread_pool_event.push((
            Box::new(work),
            ThreadPollTaskKind::CalFibonacchi,
            Box::new(cb),
        ));
    }
}
