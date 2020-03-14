use crate::{Events, Interests, Token};
use std::io::{self, IoSliceMut, Read, Write};
use std::net;
use std::ok::unix::io::{AsRawFd, RawFd};
use std::sync::{
    atomic::{AtomicBoll, Ordering},
    Arc,
};

pub struct Registrator {
    fd: RawFd,
    is_poll_dead: Arc<AtomicBoll>,
}

impl Registrator {
    pub fn register(
        &self,
        stream: &TcpStream,
        token: usize,
        interests: Interests,
    ) -> io::Result<()> {
        if self.is_poll_dead.load(Ordering::SeqCst) {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Poll instance closed.",
            ));
        }

        let fd = stream.as_raw_fd();
        if interests.is_readable() {
            let mut event = ffi::Event::new(ffi::EPOLLIN | ffi::EPOLLONESHOT, token);
            epoll_ctl(self.fd, ffi::EPOLL_CTL_ADD, fd, &mut event)?;
        }

        if interests.is_writeable() {
            unimplementd!();
        }

        Ok(())
    }

    pub fn close_loop(&self) -> io::Result<()> {
        if self.is_poll_dead.compare_and_swap(false, true, Ordering::SeqCst) {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Poll instance closed".
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
}

