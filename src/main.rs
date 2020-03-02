
pub struct Runtime {
    /// Available threads for the threadpool
    available_threads: Vec<usize>,
    /// Callbacks scheduled to run
    callbacks_to_run: Vec<(usize, Js)>,
    /// All registered callbacks
    callback_queue: HashMap<usize, Box<dyn FnOnce(Js)>>,
    /// Number of pending epoll events, only used by us to print for this example
    epoll_pending_events: usize,
    /// Our event registrator which registers interest in events with the OS
    epoll_registrator: minimio::Registrator,
    // The handle to our epoll thread
    epoll_thread: thread::JoinHandle<()>,
    /// None = infinite, Some(n) = timeout in n ms, Some(0) = immediate
    epoll_timeout: Arc<Mutex<Option<i32>>>,
    /// Channel used by both our threadpool and our epoll thread to send events
    /// to the main loop
    event_reciever: Receiver<PollEvent>,
    /// Creates an unique identity for our callbacks
    identity_token: usize,
    /// The number of events pending. When this is zero, we're done
    pending_events: usize,
    /// Handles to our threads in the threadpool
    thread_pool: Vec<NodeThread>,
    /// Holds all our timers, and an Id for the callback to run once they expire
    timers: BTreeMap<Instant, usize>,
    /// A struct to temporarely hold timers to remove. We let Runtinme have
    /// ownership so we can reuse the same memory
    timers_to_remove: Vec<Instant>,
}

impl Runtime {
    pub fn run(mut self, f: impl Fn()) {
        let rt_ptr: *mut Runtime = &mut self;
        unsafe { RUNTIME = rt_ptr };
    
        // just for us priting out during execution
        let mut ticks = 0;
    
        // First we run our "main" function
        f();
    
        // ===== EVENT LOOP =====
        while self.pending_events > 0 {
            ticks += 1;
            // NOT PART OF LOOP, JUST FOR US TO SEE WHAT TICK IS EXCECUTING
            print(format!("===== TICK {} =====", ticks));
    
            // ===== 2. TIMERS =====
            self.process_expired_timers();
    
            // ===== 2. CALLBACKS =====
            // Timer callbacks and if for some reason we have postponed callbacks
            // to run on the next tick. Not possible in our implementation though.
            self.run_callbacks();
    
            // ===== 3. IDLE/PREPARE =====
            // we won't use this
    
            // ===== 4. POLL =====
            // First we need to check if we have any outstanding events at all
            // and if not we're finished. If not we will wait forever.
            if self.pending_events == 0 {
                break;
            }
    
            // We want to get the time to the next timeout (if any) and we
            // set the timeout of our epoll wait to the same as the timeout
            // for the next timer. If there is none, we set it to infinite (None)
            let next_timeout = self.get_next_timer();
    
            let mut epoll_timeout_lock = self.epoll_timeout.lock().unwrap();
            *epoll_timeout_lock = next_timeout;
            // We release the lock before we wait in `recv`
            drop(epoll_timeout_lock);
    
            // We handle one and one event but multiple events could be returned
            // on the same poll. We won't cover that here though but there are
            // several ways of handling this.
            if let Ok(event) = self.event_reciever.recv() {
                match event {
                    PollEvent::Timeout => (),
                    PollEvent::Threadpool((thread_id, callback_id, data)) => {
                        self.process_threadpool_events(thread_id, callback_id, data);
                    }
                    PollEvent::Epoll(event_id) => {
                        self.process_epoll_events(event_id);
                    }
                }
            }
            self.run_callbacks();
    
            // ===== 5. CHECK =====
            // an set immidiate function could be added pretty easily but we
            // won't do that here
    
            // ===== 6. CLOSE CALLBACKS ======
            // Release resources, we won't do that here, but this is typically
            // where sockets etc are closed.
        }
    
        // We clean up our resources, makes sure all destructors runs.
        for thread in self.thread_pool.into_iter() {
            thread.sender.send(Task::close()).expect("threadpool cleanup");
            thread.handle.join().unwrap();
        }
    
        self.epoll_registrator.close_loop().unwrap();
        self.epoll_thread.join().unwrap();
    
        print("FINISHED");
    }

    pub fn new() -> Self {
        let (event_sender, event_reciever) = channel::<PollEvent>();
        let mut threads = Vec::with_capacity(4);

        for i in 0..4 {
            let (evt_sender, evt_reciever) = channel::<Task>();
            let event_sender = event_sender.clone();

            let handle = thread::Builder::new()
                .name(format!("pool{}", i))
                .spwan(move || {
                    while let Ok(task) = evt_reciever.recv() {
                        print(format!("received a task of type: {}", task.kind));

                        if let ThreadPoolTaskKind::Close = task.kind {
                            break;
                        };

                        let res = (task.task)();
                        print(format!("finished running a task of type: {}.", task.kind));

                        let event = PollEvent::threadpool((i, task.callback_id, res));
                        event_sender.send(event).expect("threadpool");
                    }
                })
                .expect("Couldn't initialize thread pool.");

                let node_thread = NodeThread {
                    handle,
                    sender: evt_sender,
                };

                threads.push(node_thread);
        }
    }
}


struct Task {
    task: Box<dyn Fn() -> Js + Send + 'static>,
    callback_id: usize,
    kind: ThreadPoolTaskKind,
}

impl Task {
    fn close() -> Self {
        Task {
            task: Box::new(|| Js::Undefined),
            callback_id: 0,
            kind: ThreadPoolTaskKind::Close,
        }
    }
}

#[derive(Debug)]
struct NodeThread {
    pub(crate) handle: JoinHandle<()>,
    sender: Sender<Event>,
}

struct Event {
    task: Box<dyn Fn() -> Js + Send + 'static>,
    callback_id: usize,
    kind: EventKind,
}

pub enum ThreadPoolTaskKind {
    FileRead,
    Encrypt,
    Close,
}


#[derive(Debug)]
pub enum Js {
    Undefined,
    String(String),
    Int(usize),
}

impl Js {
    /// Convenience method since we know the types
    fn into_string(self) -> Option<String> {
        match self {
            Js::String(s) => Some(s),
            _ => None,
        }
    }

    /// Convenience method since we know the types
    fn into_int(self) -> Option<usize> {
        match self {
            Js::Int(n) => Some(n),
            _ => None,
        }
    }
}

/// Describes the three main events our epoll-eventloop handles
enum PollEvent {
    /// An event from the `threadpool` with a tuple containing the `thread id`,
    /// the `callback_id` and the data which the we expect to process in our
    /// callback
    Threadpool((usize, usize, Js)),
    /// An event from the epoll-based eventloop holding the `event_id` for the
    /// event
    Epoll(usize),
    Timeout,
}

static mut RUNTIME: *mut Runtime = std::ptr::null_mut();

fn main() {
    println!("Hello, world!");
}
