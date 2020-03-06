use std::{
    collections::HashMap,
    sync::mpsc::{channel, Receiver},
    thread,
};

use crate::nodethread::NodeThread;
use crate::pollevent::PollEvent;
use crate::task::{Task, ThreadPollTaskKind};
use crate::ioresult::IOResult;

pub(crate) static mut RUNTIME: *mut Runtime = std::ptr::null_mut();

pub struct Runtime {
    // Pending callbacks
    callback_pending: HashMap<usize, Box<dyn FnOnce(IOResult)>>,
    // Ready callbacks
    callback_ready: Vec<(usize, IOResult)>,
    // The unique id for callback function
    callback_token: usize,
    // Pending events
    event_pending: usize,
    // event_queue
    pub(crate) thread_pool_event: Vec<(
        Box<dyn Fn() -> IOResult + Send + 'static>,
        ThreadPollTaskKind,
        Box<dyn FnOnce(IOResult) + 'static>,
    )>,
    // Event reciever
    event_reciever: Receiver<PollEvent>,
    // Available threads in thread_pool
    thread_available: Vec<usize>,
    // Thread pool
    thread_pool: Vec<NodeThread>,
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

        Runtime {
            event_reciever: event_reciever,
            event_pending: 0,
            thread_pool_event: vec![],
            callback_pending: HashMap::new(),
            callback_ready: vec![],
            callback_token: 0,
            thread_available: (0..4).collect(),
            thread_pool: thread_pool,
        }
    }

    pub fn run(mut self, async_func: impl Fn()) {
        let rt_ptr: *mut Runtime = &mut self;
        unsafe { RUNTIME = rt_ptr };

        let mut main_ticks = 0;

        async_func();

        while self.thread_pool_event.len() > 0 {
            // 0. Output the main loop
            main_ticks += 1;
            println!("===== MAIN LOOP {} =====", main_ticks);

            self.register_threadpool_event();

            while self.event_pending > 0 {
                // NOT PART OF LOOP, JUST FOR US TO SEE WHAT TICK IS EXCECUTING
                // ===== 4. POLL =====
                // First we need to check if we have any outstanding events at all
                // and if not we're finished. If not we will wait forever.
                if let Ok(event) = self.event_reciever.recv() {
                    match event {
                        PollEvent::Threadpool((thread_id, callback_id, data)) => {
                            self.process_threadpool_event(thread_id, callback_id, data);
                        }
                    }
                }
                self.run_callbacks();
            }
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

    fn generate_cb_identity(&mut self) -> usize {
        loop {
            let ident = self.generate_identity();
            if !self.callback_pending.contains_key(&ident) {
                break ident;
            }
        }
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
                self.event_pending += 1;
            } else {
                break;
            }
        }
    }

    fn process_threadpool_event(
        &mut self,
        thread_id: usize,
        callback_id: usize,
        data: IOResult,
    ) {
        self.callback_ready.push((callback_id, data));
        self.thread_available.push(thread_id);
    }

    fn run_callbacks(&mut self) {
        while let Some((callback_id, data)) = self.callback_ready.pop() {
            let cb = self.callback_pending.remove(&callback_id).unwrap();
            cb(data);
            self.event_pending -= 1;
        }
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
}
