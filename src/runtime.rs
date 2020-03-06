use std::{ 
    thread, 
    collections::HashMap, 
    sync::mpsc::{channel, Receiver, Sender}, 
};

use crate::nodethread::NodeThread;
use crate::pollevent::PollEvent;
use crate::task::{
    Task, 
    ThreadPollTaskKind,
};

pub(crate) static mut RUNTIME: *mut Runtime = std::ptr::null_mut();

pub struct Runtime {
    /// Available threads for the threadpool
    available_threads: Vec<usize>,
    // Callbacks scheduled to run

    /// All registered callbacks
    callback_queue: HashMap<usize, Box<dyn FnOnce(Option<String>) >>,
    // the unique id for callback funciton
    identity_token: usize,
    pending_events: usize,
    thread_pool: Vec<NodeThread>,
    event_reciever: Receiver<PollEvent>,
    callbacks_to_run: Vec<(usize, Option<String>)>,
}

impl Runtime {
    pub fn new() -> Self {
        // main thread
        let (event_sender, event_reciever) = channel::<PollEvent>();
        let mut threads = Vec::with_capacity(4);

        for i in 0..4 {
            let (evt_sender, evt_reciever) = channel::<Task>();
            let event_sender = event_sender.clone();

            let handle = thread::Builder::new()
                .name(format!("pool{}", i))
                .spawn( move || {
                    while let Ok(task) = evt_reciever.recv() {
                        println!("received a task of type: {}", task.kind);

                        let res = (task.task)();
                        println!("finished running a task of type: {}.", task.kind);

                        let event = PollEvent::Threadpool((i, task.callback_id, res));
                        event_sender.send(event).expect("threadpool");
                    }
                })
                .expect("Couldn't initialze thread pool");

            let node_thread = NodeThread {
                handle,
                sender: evt_sender,
            };

            threads.push(node_thread);
        }

        Runtime {
            available_threads: (0..4).collect(),
            callback_queue: HashMap::new(),
            identity_token: 0,
            pending_events: 0,
            thread_pool: threads,
            event_reciever: event_reciever,
            callbacks_to_run: vec![],
        }
    }

    pub fn run(mut self, f: impl Fn()) {

        let rt_ptr: *mut Runtime = &mut self;
        unsafe { RUNTIME = rt_ptr };

        let mut ticks = 0;

        f();

        while self.pending_events > 0 {
            ticks += 1;
            // NOT PART OF LOOP, JUST FOR US TO SEE WHAT TICK IS EXCECUTING
            //println!("===== TICK {} =====", ticks);

            // ===== 4. POLL =====
            // First we need to check if we have any outstanding events at all
            // and if not we're finished. If not we will wait forever.

            if let Ok(event) = self.event_reciever.recv() {
                match event {
                    PollEvent::Threadpool((thread_id, callback_id, data)) => {
                        self.process_threadpool_events(thread_id, callback_id, data);
                    }
                }
            }

            self.run_callbacks();
        }

    }

    fn add_callback<U>(&mut self, ident: usize, cb: U) 
    where U: FnOnce(Option<String>) + 'static {
        self.callback_queue.insert(ident, Box::new(cb));
    }

    fn generate_identity(&mut self) -> usize {
        self.identity_token = self.identity_token.wrapping_add(1);
        self.identity_token
    }

    fn generate_cb_identity(&mut self) -> usize {
        loop {
            let ident = self.generate_identity();
            if !self.callback_queue.contains_key(&ident) {
                break ident;
            }
        }
    }

    fn get_available_thread(&mut self) -> usize {
        match self.available_threads.pop() {
            Some(thread_id) => thread_id,
            None => panic!("Out of threads"),
        }
    }

    pub fn register_event_threadpool<T, U>(&mut self, task: T, kind: ThreadPollTaskKind, cb: U)
    where 
        T: Fn() -> Option<String> + Send + 'static,
        U: FnOnce(Option<String>) + 'static 
    {
        let callback_id = self.generate_cb_identity();
        self.add_callback(callback_id, cb);

        let event = Task {
            task: Box::new(task),
            callback_id: callback_id,
            kind: kind, 
        };

        let available_thread = self.get_available_thread();
        self.thread_pool[available_thread]
            .sender
            .send(event)
            .expect("register work");
        self.pending_events += 1;
    }

    fn process_threadpool_events(&mut self, thread_id: usize, callback_id: usize, data: Option<String>) {
        self.callbacks_to_run.push((callback_id, data));
        self.available_threads.push(thread_id);
    }

    fn run_callbacks(&mut self) {
        while let Some((callback_id, data)) = self.callbacks_to_run.pop() {
            let cb = self.callback_queue.remove(&callback_id).unwrap();
            cb(data);
            self.pending_events -= 1;
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
    };
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