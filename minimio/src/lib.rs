use std::{sync::mpsc::Sender, thread::JoinHandle};

struct Reactor {
    handle: Option<JoinHandle<()>>,
    //registerator: Option<Registerator>,
}

impl Reactor {
    fn new(evt_sender: Sender<usize>) -> Self {
        Reactor { handle: None }
    }
}

#[repr(C)] // store data in memory as C language
pub union Data {
    void: *const std::os::raw::c_void,
    fd: i32,
    uint32: u32,
    uint64: u64,
}

pub mod ffi {
    pub const EPOLL_CTL_ADD: i32 = 1;
    pub const EPOLLIN: i32 = 0x1;
    pub const EPOLLONESHOT: i32 = 0x40000000;

    #[link(name = "c")]
    extern "C" {
        pub fn close(fd: i32) -> i32;
        pub fn epoll_create(size: i32) -> i32;
        pub fn epoll_ctl(epfd: i32, op: i32, fb: i32, event: *mut Event) -> i32;
        pub fn epoll_wait(epfd: i32, events: *mut Event, maxevents: i32, timeout: i32) -> i32;
    }

    #[derive(Debug)]
    #[repr(C)]
    pub struct Event {
        pub events: u32,
        pub epoll_data: usize,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
