use std::io::{self, IoSliceMut, Read, Write};
use std::net;
use std::os::unix::io::{AsRawFd, RawFd};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::{Events, Interests, Token};

pub type Event = ffi::Event;

pub struct Registrator {
    fd: RawFd,
    is_poll_dead: Arc<AtomicBool>,
}

impl Registrator {
    pub fn register(
        &self,
        stream: &TcpStream,
        token: usize,
        interest: Interests,
    ) -> io::Result<()> {
        if self.is_poll_dead.load(Ordering::SeqCst) {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Poll instance closed.",
            ));
        }
        let fd = stream.as_raw_fd();
        if interest.is_readable() {
            let mut event = ffi::Event::new(ffi::EPOLLIN | ffi::EPOLLONESHOT, token);
            epoll_ctl(self.fd, ffi::EPOLL_CTL_ADD, fd, &mut event)?;
        };

        if interest.is_writeable() {
            unimplemented!();
        }
        Ok(())
    }

    pub fn close_loop(&self) -> io::Result<()> {
        if self
            .is_poll_dead
            .compare_and_swap(false, true, Ordering::SeqCst)
        {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Poll instance close.",
            ));
        }

        let wake_fd = eventfd(1, 0)?;
        let mut event = ffi::Event::new(ffi::EPOLLIN, 0);
        epoll_ctl(self.fd, ffi::EPOLL_CTL_ADD, wake_fd, &mut event)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct Selector {
    fd: RawFd,
}

impl Selector {
    pub fn new() -> io::Result<Self> {
        Ok(Selector {
            fd: epoll_create()?,
        })
    }

    // This function blocks and waits until an event has been recieved. 'timeout' None means
    // the poll will never time out.
    pub fn select(&self, events: &mut Events, timeout_ms: Option<i32>) -> io::Result<()> {
        events.clear();
        let timeout = timeout_ms.unwrap_or(-1);
        epoll_wait(self.fd, events, 1024, timeout).map(|n_events| {
            unsafe { events.set_len(n_events as usize) };
        })
    }

    pub fn registrator(&self, is_poll_dead: Arc<AtomicBool>) -> Registrator {
        Registrator {
            fd: self.fd,
            is_poll_dead: is_poll_dead,
        }
    }
}

impl Drop for Selector {
    fn drop(&mut self) {
        match close_fd(self.fd) {
            Ok(_) => (),
            Err(e) => {
                if !std::thread::panicking() {
                    panic!(e);
                }
            }
        }
    }
}

pub struct TcpStream {
    inner: net::TcpStream,
}

impl TcpStream {
    pub fn connect(addr: impl net::ToSocketAddrs) -> io::Result<Self> {
        // actually we should set this to non-blocking before we call connect which is not something
        // we get from the stdlib but could do with a syscall. Let's skip that step in this example.
        // In other words this will block shortly establishing a connection to the remote server
        let stream = net::TcpStream::connect(addr)?;
        stream.set_nonblocking(true)?;
        Ok(TcpStream { inner: stream })
    }
}

impl Read for TcpStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.set_nonblocking(false)?;
        (&self.inner).read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut]) -> io::Result<usize> {
        (&self.inner).read_vectored(bufs)
    }
}

impl Write for TcpStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl AsRawFd for TcpStream {
    fn as_raw_fd(&self) -> RawFd {
        self.inner.as_raw_fd()
    }
}

mod ffi {
    use std::io;
    use std::os::raw::c_void;

    pub const EPOLL_CTL_ADD: i32 = 1;
    pub const EPOLL_CTL_DEL: i32 = 2;
    pub const EPOLLIN: i32 = 0x1;
    pub const EPOLLONESHOT: i32 = 0x40000000;

    #[repr(C)]
    pub struct Event {
        events: u32,
        epoll_data: usize,
    }

    impl Event {
        pub fn new(events: i32, id: usize) -> Self {
            Event {
                events: events as u32,
                epoll_data: id,
            }
        }

        pub fn data(&self) -> usize {
            self.epoll_data
        }

        pub fn id(&self) -> super::Token {
            self.data()
        }
    }

    #[link(name = "c")]
    extern "C" {
        /// http://man7.org/linux/man-pages/man2/epoll_create1.2.html
        pub fn epoll_create(size: i32) -> i32;

        /// http://man7.org/linux/man-pages/man2/close.2.html
        pub fn close(fd: i32) -> i32;

        /// http://man7.org/linux/man-pages/man2/epoll_ctl.2.html
        pub fn epoll_ctl(epfd: i32, op: i32, fd: i32, event: *mut Event) -> i32;

        /// http://man7.org/linux/man-pages/man2/epoll_wait.2.html
        ///
        /// - epoll_event is a pointer to an array of Events
        /// - timeout of -1 means indefinite
        pub fn epoll_wait(epfd: i32, events: *mut Event, maxevents: i32, timeout: i32) -> i32;

        /// http://man7.org/linux/man-pages/man2/timerfd_create.2.html
        pub fn eventfd(initva: u32, flags: i32) -> i32;
    }
}

fn epoll_create() -> io::Result<i32> {
    // Size is ignored but must be greater than zero
    let res = unsafe { ffi::epoll_create(1) };
    if res < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(res)
    }
}

fn close_fd(fd: i32) -> io::Result<()> {
    let res = unsafe { ffi::close(fd) };
    if res < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn epoll_ctl(epfd: i32, op: i32, fd: i32, event: &mut ffi::Event) -> io::Result<()> {
    let res = unsafe { ffi::epoll_ctl(epfd, op, fd, event) };
    if res < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn epoll_wait(epfd: i32, events: &mut [Event], maxevents: i32, timeout: i32) -> io::Result<i32> {
    let res = unsafe { ffi::epoll_wait(epfd, events.as_mut_ptr(), maxevents, timeout) };
    if res < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(res)
    }
}

fn eventfd(initva: u32, flags: i32) -> io::Result<i32> {
    let res = unsafe { ffi::eventfd(initva, flags) };
    if res < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(res)
    }
}
