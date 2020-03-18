use std::{
    fmt, fs,
    io::{Read, Write},
    thread,
    time::{Duration, Instant},
};

use crate::ioresult::IOResult;
use minimio;

pub enum ThreadPoolTaskKind {
    Close,
    FileRead,
    CalFibonacchi,
}

impl fmt::Display for ThreadPoolTaskKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ThreadPoolTaskKind::Close => write!(f, "Close"),
            ThreadPoolTaskKind::FileRead => write!(f, "FileRead"),
            ThreadPoolTaskKind::CalFibonacchi => write!(f, "CalFibonacchi"),
        }
    }
}

pub struct Task {
    pub(crate) task: Box<dyn Fn() -> IOResult + Send + 'static>,
    pub(crate) callback_id: usize,
    pub(crate) kind: ThreadPoolTaskKind,
}

impl Task {
    pub fn close() -> Self {
        Task {
            task: Box::new(|| IOResult::Undefined),
            callback_id: 0,
            kind: ThreadPoolTaskKind::Close,
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
            .push((Box::new(work), ThreadPoolTaskKind::FileRead, Box::new(cb)));
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
            ThreadPoolTaskKind::CalFibonacchi,
            Box::new(cb),
        ));
    }
}

pub struct Http;
impl Http {
    pub fn http_get_slow(url: &str, delay_ms: u32, cb: impl Fn(IOResult) + 'static + Clone) {
        let rt = unsafe { &mut *crate::runtime::RUNTIME };

        let adr = "slowwly.robertomurray.co.uk:80";
        let mut stream = minimio::TcpStream::connect(adr).unwrap();

        let request = format!(
            "GET /delay/{}/url/http://{} HTTP/1.1\r\n\
             Host: slowwly.robertomurray.co.uk\r\n\
             Connection: close\r\n\
             \r\n",
            delay_ms, url
        );

        stream
            .write_all(request.as_bytes())
            .expect("Error writing to stream");

        let token = rt.generate_cb_identity();
        rt.epoll_registrator
            .register(&mut stream, token, minimio::Interests::READABLE)
            .unwrap();

        let wrapped = move |_n| {
            let mut stream = stream;
            let mut buffer = String::new();
            stream
                .read_to_string(&mut buffer)
                .expect("Stream read error");
            cb(IOResult::String(buffer));
        };

        rt.register_epoll_event(token, wrapped);
    }
}

pub struct Timeout;
impl Timeout {
    pub fn set_timeout(ms: u64, cb: impl Fn(IOResult) + 'static) {
        let rt = unsafe { &mut *crate::runtime::RUNTIME };
        rt.set_timeout(ms, cb);
    }
}
