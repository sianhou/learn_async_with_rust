use minimio::{Events, Interests, Poll, Registrator, TcpStream};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::{io, io::Read, io::Write, thread, thread::JoinHandle};

const TEST_TOKEN: usize = 10; // Hard coded for this test only

fn main() {
    let (evt_sender, evt_reciever) = channel();
    let mut reactor = Reactor::new(evt_sender);
    let mut executor = Excutor::new(evt_reciever);

    let mut stream = TcpStream::connect("34.255.19.16:80").unwrap();
    let request = b"GET /delay/1000/url/http://www.baidu.com HTTP/1.1\r\nHost: slowwly.robertomurray.co.uk\r\nConnection: close\r\n\r\n";

    stream.write_all(request).expect("Stream write err.");

    let registrator = reactor.registrator();
    registrator.register(&mut stream, TEST_TOKEN, Interests::READABLE);

    executor.suspend(TEST_TOKEN, move || {
        let mut buffer = String::new();
        stream.read_to_string(&mut buffer).unwrap();
        println!("{}",buffer);
        assert!(!buffer.is_empty(), "Got an empty buffer");
        registrator.close_loop().expect("close loop err.");
    });

    executor.block_all();
}

struct Reactor {
    handle: Option<JoinHandle<()>>,
    registrator: Option<Registrator>,
}

impl Reactor {
    fn new(evt_sender: Sender<usize>) -> Reactor {
        let mut poll = Poll::new().unwrap();
        let registrator = poll.registrator();

        let handle = thread::spawn(move || {
            let mut events = Events::with_capacity(1024);
            loop {
                match poll.poll(&mut events, Some(200)) {
                    Ok(..) => (),
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => break,
                    Err(e) => panic!("Poll error: {:?}, {}", e.kind(), e),
                };
                for event in &events {
                    let event_token = event.id();
                    evt_sender.send(event_token).expect("send event_token err.");
                }
            }
        });

        Reactor {
            handle: Some(handle),
            registrator: Some(registrator),
        }
    }

    fn registrator(&mut self) -> Registrator {
        self.registrator.take().unwrap()
    }
}

impl Drop for Reactor {
    fn drop(&mut self) {
        let handle = self.handle.take().unwrap();
        handle.join().unwrap();
    }
}

struct Excutor {
    events: Vec<(usize, Box<dyn FnMut()>)>,
    evt_reciever: Receiver<usize>,
}

impl Excutor {
    fn new(evt_reciever: Receiver<usize>) -> Excutor {
        Excutor {
            events: vec![],
            evt_reciever: evt_reciever,
        }
    }

    fn suspend(&mut self, id: usize, f: impl FnMut() + 'static) {
        self.events.push((id, Box::new(f)));
    }

    fn resume(&mut self, event: usize) {
        let (_, f) = self
            .events
            .iter_mut()
            .find(|(e, _)| *e == event)
            .expect("Couldn't find event.");
        f();
    }

    fn block_all(&mut self) {
        while let Ok(recieved_token) = self.evt_reciever.recv() {
            assert_eq!(TEST_TOKEN, recieved_token, "Non matching tokens.");
            self.resume(recieved_token);
        }
    }
}
