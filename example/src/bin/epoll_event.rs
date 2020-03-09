use std::{
    io::{self, Write},
    net::TcpStream,
    os::unix::io::AsRawFd,
};

use minimio::ffi;

fn main() {
    // A counter to keep track of how many events we're expecting to act on
    let mut event_counter = 0;

    // First we create the event queue.
    // The size argument is ignored but needs to be larger than 0
    let queue = unsafe { ffi::epoll_create(1) };

    // This is how we basically check for errors and handle them using most
    // C APIs
    // We handle them by just panicing here in out example.
    if queue < 0 {
        panic!(io::Error::last_os_error());
    }

    // As you'll see below, we need a place to store the streams so they're
    // not closed
    let mut streams = vec![];

    // We create 5 requests to an endpoint we control the delay on
    for i in 0..5 {
        // This site has an api to simulate slow responses from a server
        let addr = "slowwly.robertomurray.co.uk:80";
        let mut stream = TcpStream::connect(addr).unwrap();

        // The delay is passed in to the GET request as milliseconds.
        // We'll create delays in decending order so we should recieve
        // them as '5, 4, 3, 2, 1'
        let delay = (4 - i) * 1000;
        let request = format!(
            "GET /delay/{}/url/http://www.baidu.com HTTP/1.1\r\n\
             Host: slowwly.robertomurray.co.uk\r\n\
             Connection: close\r\n\
             \r\n",
            delay
        );
        stream.write_all(request.as_bytes()).unwrap();

        // make this socket non-blocking. Well, not really needed since
        // we're not using it in this example...
        stream.set_nonblocking(true).unwrap();

        // Then register interest in getting notified for 'Read' events on
        // this socket. The 'Event' struct is where we specify what events
        // we want to register interest in and other configurations using
        // flags.
        //
        // 'EPOLLIN' is interest in 'Read' events.
        // 'EPOLLONESHOT' means that we remove any interests from the queue
        // after first event. If we don't do that we need to 'deregister'
        // our interest manually when we're done with the socket.
        //
        // 'epoll_data' is user provided data, so we can put a pointer or an
        // integer value there to identify the event. We just use 'i' which
        // is the loop count to indentify the events.
        let mut event = ffi::Event {
            events: (ffi::EPOLLIN | ffi::EPOLLONESHOT) as u32,
            epoll_data: i,
        };

        println!("{:?}",event);

        // This is the call where we actually "ADD" an interest to our queue.
        // 'EPOLL_CTL_ADD' is the flag which controls whether we want to add
        // interest, modify an existing one or remove interests from the queue.
        let op = ffi::EPOLL_CTL_ADD;
        let res = unsafe { ffi::epoll_ctl(queue, op, stream.as_raw_fd(), &mut event) };
        if res < 0 {
            panic!(io::Error::last_os_error());
        }

        // Letting 'stream' go out of scope in Rust automatically runs
        // its destructor which closes the socket. We prevent that by
        // holding on to it until we're finished
        streams.push(stream);
        event_counter += 1;
    }

    // Now we wait for events
    while event_counter > 0 {
        let mut events = Vec::with_capacity(10);

        let res = unsafe {ffi::epoll_wait(queue, events.as_mut_ptr(), 10, -1)};

        if res < 0 {
            panic!(io::Error::last_os_error());
        };

        unsafe { events.set_len(res as usize) };
        for event in events {
            println!("RECIEVED: {:?}", event);
            event_counter -= 1;
        }
    }

    let res = unsafe { ffi::close(queue) };
    if res < 0 {
        panic!(io::Error::last_os_error());
    }
    println!("FINISHED");
}
