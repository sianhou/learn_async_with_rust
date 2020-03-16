use minimio::{Events, Interests, Poll, TcpStream};
use std::io::{self, Read, Write};
use std::sync::mpsc::channel;
use std::thread;

#[test]
fn multiple_registraions() {
    let mut poll = Poll::new().unwrap();
    let registrator = poll.registrator();

    let (evt_sender, evt_reciever) = channel();

    let mut rt = Runtime { events: vec![] };

    let provided_token = 10;
    let provided_token2 = 11;

    let handle = thread::spawn(move || {
        let mut events = Vec::with_capacity(1024);
        loop {
            println!("POLLING");
            let will_close = false;
            println!("{:?}", poll);
            match poll.poll(&mut events, Some(200)) {
                Ok(..) => (),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {
                    println!("INTERRUPTED: {}", e);
                    break;
                }
                Err(e) => panic!("Poll error: {:?}, {}", e.kind(), e),
            };

            for event in &events {
                let event_token = event.id();
                println!("GOT EVENT: {:?}", event_token);

                evt_sender.send(event_token).expect("send event_token err.");
            }

            if will_close {
                break;
            }
        }
    });

    let mut stream = TcpStream::connect("slowwly.robertomurray.co.uk:80").unwrap();
    let request = "GET /delay/1000/url/http://www.baidu.com HTTP/1.1\r\n\
                   Host: slowwly.robertomurray.co.uk\r\n\
                   Connection: close\r\n\
                   \r\n";
    stream
        .write_all(request.as_bytes())
        .expect("Error writing to stream");

    let mut stream2 = TcpStream::connect("slowwly.robertomurray.co.uk:80").unwrap();

    let request2 = "GET /delay/1000/url/http://www.baidu.com HTTP/1.1\r\n\
                    Host: slowwly.robertomurray.co.uk\r\n\
                    Connection: close\r\n\
                    \r\n";
    stream2
        .write_all(request2.as_bytes())
        .expect("Error writing to stream");

    registrator
        .register(&mut stream, provided_token, Interests::READABLE)
        .expect("registration err.");

    registrator
        .register(&mut stream2, provided_token2, Interests::READABLE)
        .expect("registration err.");
    println!("HERE");
    rt.spawn(provided_token, move || {
        let mut buffer = String::new();
        stream.read_to_string(&mut buffer).unwrap();
        assert!(!buffer.is_empty(), "Got an empty buffer");
        println!("PROPOSED API:\n{}", buffer);
    });

    rt.spawn(provided_token2, move || {
        let mut buffer = String::new();
        stream2.read_to_string(&mut buffer).unwrap();
        assert!(!buffer.is_empty(), "Got an empty buffer");
        println!("PROPOSED API:\n{}", buffer);
    });

    println!("WATTING FOR EVENTS");
    let mut counter = 0;
    while let Ok(recieved_token) = evt_reciever.recv() {
        counter += 1;
        println!("RECIEVED EVENT: {:?}", recieved_token);
        rt.run(recieved_token);

        if counter == 2 {
            registrator.close_loop().expect("close loop err.");
        }
    }
    handle.join().expect("error joining thread");
    println!("EXITING");
}

struct Runtime {
    events: Vec<(usize, Box<dyn FnMut()>)>,
}

impl Runtime {
    fn spawn(&mut self, id: usize, f: impl FnMut() + 'static) {
        self.events.push((id, Box::new(f)));
    }

    fn run(&mut self, event: usize) {
        println!("RUNNING EVENT: {}", event);
        let (_, f) = self
            .events
            .iter_mut()
            .find(|(e, _)| *e == event)
            .expect("Counldn't find event");

        f();
    }
}
