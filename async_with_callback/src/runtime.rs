use std::{
    collections::{BTreeMap, HashMap},
    io,
    sync::{
        mpsc::{channel, Receiver},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use crate::ioresult::IOResult;
use crate::nodethread::NodeThread;
use crate::pollevent::PollEvent;
use crate::task::{Task, ThreadPoolTaskKind};
use minimio;

pub(crate) static mut RUNTIME: *mut Runtime = std::ptr::null_mut();

pub struct Runtime {
    // Pending callbacks
    callback_pending: HashMap<usize, Box<dyn FnOnce(IOResult)>>,
    // Ready callbacks
    callback_ready: Vec<(usize, IOResult)>,
    // The unique id for callback function
    callback_token: usize,
    ///
    pub epoll_registrator: minimio::Registrator,
    ///
    epoll_thread: thread::JoinHandle<()>,
    // Pending events
    pending_events: usize,
    ///
    event_epoll_pending: usize,
    // event_queue
    pub(crate) thread_pool_event: Vec<(
        Box<dyn Fn() -> IOResult + Send + 'static>,
        ThreadPoolTaskKind,
        Box<dyn FnOnce(IOResult) + 'static>,
    )>,
    // Event reciever
    event_reciever: Receiver<PollEvent>,
    // Available threads in thread_pool
    thread_available: Vec<usize>,
    // Thread pool
    thread_pool: Vec<NodeThread>,
    ///
    timers: BTreeMap<Instant, usize>,
    ///
    timers_to_remove: Vec<Instant>,
}

impl Runtime {
    pub fn new() -> Self {
        // main thread
        let (event_sender, event_reciever) = channel::<PollEvent>();
        let mut thread_pool = Vec::with_capacity(4);

        for i in 0..4 {
            let (evt_sender, evt_reciever) = channel::<Task>();
            let event_sender = event_sender.clone();
            let thread_id = i;

            let handle = thread::spawn(move || {
                while let Ok(task) = evt_reciever.recv() {
                    println!(
                        "thread {} - received a task of type: {}",
                        thread_id, task.kind
                    );

                    if let ThreadPoolTaskKind::Close = task.kind {
                        println!("thread {} - closed", thread_id);
                        break;
                    }

                    let res = (task.task)();
                    println!(
                        "thread {} - finished running a task of type: {}.",
                        thread_id, task.kind
                    );

                    let event = PollEvent::Threadpool((i, task.callback_id, res));
                    event_sender.send(event).expect("threadpool");
                }
            });

            let node_thread = NodeThread {
                handle: handle,
                sender: evt_sender,
            };

            thread_pool.push(node_thread);
        }

        // -------- epoll thread --------
        let mut poll = minimio::Poll::new().expect("Error creating epoll queue");
        let registrator = poll.registrator();
        let epoll_timeout = Arc::new(Mutex::new(None));
        let epoll_timeout_clone = epoll_timeout.clone();

        let epoll_thread = thread::spawn(move || {
            let mut events = minimio::Events::with_capacity(1024);

            loop {
                let epoll_timeout_handle = epoll_timeout_clone.lock().unwrap();
                let timeout = *epoll_timeout_handle;
                drop(epoll_timeout_handle);

                match poll.poll(&mut events, timeout) {
                    Ok(v) if v > 0 => {
                        for i in 0..v {
                            let event = events.get_mut(i).expect("No events in event list.");
                            println!("epoll event {} is ready", event.id());

                            let event = PollEvent::Epoll(event.id() as usize);
                            event_sender.send(event).expect("epoll event");
                        }
                    }
                    Ok(v) if v == 0 => {
                        println!("epoll event timeout is ready");
                        event_sender
                            .send(PollEvent::Timeout)
                            .expect("epoll timeout");
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {
                        println!("received event of type: Close");
                        break;
                    }
                    Err(e) => panic!("{:?}", e),
                    _ => unreachable!(),
                }
            }
        });

        Runtime {
            event_reciever: event_reciever,
            epoll_registrator: registrator,
            epoll_thread: epoll_thread,
            pending_events: 0,
            event_epoll_pending: 0,
            thread_pool_event: vec![],
            callback_pending: HashMap::new(),
            callback_ready: vec![],
            callback_token: 0,
            thread_available: (0..4).collect(),
            thread_pool: thread_pool,
            timers: BTreeMap::new(),
            timers_to_remove: vec![],
        }
    }

    pub fn run(mut self, async_func: impl Fn()) {
        let rt_ptr: *mut Runtime = &mut self;
        unsafe { RUNTIME = rt_ptr };

        let mut main_ticks = 0;

        async_func();

        while self.pending_events > 0 {
            // 0. Output the main loop
            main_ticks += 1;
            println!("===== MAIN LOOP {} =====", main_ticks);

            if self.thread_pool_event.len() > 0 {
                self.register_threadpool_event();
            }

            self.process_expired_timers();

            self.run_callbacks();

            if self.pending_events == 0 {
                break;
            }

            // NOT PART OF LOOP, JUST FOR US TO SEE WHAT TICK IS EXCECUTING
            // ===== 4. POLL =====
            // First we need to check if we have any outstanding events at all
            // and if not we're finished. If not we will wait forever.
            if let Ok(event) = self.event_reciever.recv() {
                match event {
                    PollEvent::Threadpool((thread_id, callback_id, data)) => {
                        self.process_threadpool_event(thread_id, callback_id, data);
                    }
                    PollEvent::Epoll(event_id) => {
                        self.process_epoll_event(event_id);
                    }
                    PollEvent::Timeout => (),
                }
            }
            self.run_callbacks();
        }

        // Close the threadpool
        for thread in self.thread_pool.into_iter() {
            thread
                .sender
                .send(Task::close())
                .expect("threadpool cleanup");
            thread.handle.join().unwrap();
        }
    }

    fn add_callback<U>(&mut self, ident: usize, cb: U)
    where
        U: FnOnce(IOResult) + 'static,
    {
        self.callback_pending.insert(ident, Box::new(cb));
    }

    fn generate_identity(&mut self) -> usize {
        self.callback_token = self.callback_token.wrapping_add(1);
        self.callback_token
    }

    pub fn generate_cb_identity(&mut self) -> usize {
        loop {
            let ident = self.generate_identity();
            if !self.callback_pending.contains_key(&ident) {
                break ident;
            }
        }
    }

    fn process_expired_timers(&mut self) {
        let timers_to_remove = &mut self.timers_to_remove;

        self.timers
            .range(..=Instant::now())
            .for_each(|(k, _)| timers_to_remove.push(*k));

        while let Some(key) = self.timers_to_remove.pop() {
            let callback_id = self.timers.remove(&key).unwrap();
            self.callback_ready.push((callback_id, IOResult::Undefined));
        }
    }

    fn get_next_timer(&self) -> Option<i32> {
        self.timers.iter().nth(0).map(|(&instant, _)| {
            let mut time_to_next_timeout = instant - Instant::now();
            if time_to_next_timeout < Duration::new(0, 0) {
                time_to_next_timeout = Duration::new(0, 0);
            }
            time_to_next_timeout.as_millis() as i32
        })
    }

    fn register_threadpool_event(&mut self) {
        loop {
            if self.thread_pool_event.len() > 0 && self.thread_available.len() > 0 {
                let (task, kind, cb) = self.thread_pool_event.pop().unwrap();

                let callback_id = self.generate_cb_identity();
                self.add_callback(callback_id, cb);

                let thread_id = self.thread_available.pop().unwrap();
                let event = Task {
                    task: task,
                    callback_id: callback_id,
                    kind: kind,
                };
                self.thread_pool[thread_id]
                    .sender
                    .send(event)
                    .expect("register work");
                self.pending_events += 1;
            } else {
                break;
            }
        }
    }

    pub fn register_epoll_event(&mut self, token: usize, cb: impl FnOnce(IOResult) + 'static) {
        self.add_callback(token, cb);
        println!("Event with id: {} registered.", token);
        self.pending_events += 1;
        self.event_epoll_pending += 1;
    }

    fn process_threadpool_event(&mut self, thread_id: usize, callback_id: usize, data: IOResult) {
        self.callback_ready.push((callback_id, data));
        self.thread_available.push(thread_id);
    }

    fn process_epoll_event(&mut self, event_id: usize) {
        self.callback_ready.push((event_id, IOResult::Undefined));
        self.pending_events -= 1;
    }

    fn run_callbacks(&mut self) {
        while let Some((callback_id, data)) = self.callback_ready.pop() {
            let cb = self.callback_pending.remove(&callback_id).unwrap();
            cb(data);
            self.pending_events -= 1;
        }
    }

    pub fn set_timeout(&mut self, ms: u64, cb: impl Fn(IOResult) + 'static) {
        let now = Instant::now();

        let cb_id = self.generate_cb_identity();
        self.add_callback(cb_id, cb);

        let timeout = now + Duration::from_millis(ms);
        self.timers.insert(timeout, cb_id);

        self.pending_events += 1;
        println!("Registered timer event id: {}", cb_id);
    }
}

#[test]
fn test_generate_cb_identity() {
    let mut queue: HashMap<usize, String> = HashMap::new();
    queue.insert(0, "0".to_string());
    queue.insert(1, "1".to_string());
    queue.insert(2, "2".to_string());

    let mut ident = 0;
    loop {
        if !queue.contains_key(&ident) {
            break;
        } else {
            ident += 1;
        }
    }
    assert_eq!(ident, 3);
    /*
        let mut ident = 0;
        let taken = queue.contains_key(&ident);
        if !taken {
        } else {
            loop {
                ident += 1;
                if queue.contains_key(&ident) {
                    break;
                }
            }
        }
        assert_eq!(ident, 3);
    */
}
