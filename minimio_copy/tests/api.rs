use minimio::{Events, Interests, Poll, Registrator, TcpStream};
use std::{io, io::Read, io::Write, thread, thread::JoinHandle};
use std::sync::mpsc::{channel, Receiver, Sender};

const TESK_TOKEN: usize = 10;

#[test]
fn proposed_api() {
}

struct Reactor {
    handle: Option<JoinHandle<()>>,
    registrator: Option<Registrator>,
}

impl Reactor {
    fn new(evt_sender: Sender<usize>) -> Reactor {
         let mut poll = Poll::new().unwrap();
         let registrator = poll.registrator();

        let handle = thread::spwan(move || {
            let mut events = Events::with_capacity(1024);
        }
             
        );
    }

}

