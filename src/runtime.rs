use std::{ 
    thread, 
    collections::HashMap, 
    sync::mpsc::channel 
};

use crate::nodethread::NodeThread;
use crate::pollevent::PollEvent;
use crate::task::{
    Task, 
    ThreadPollTaskKind
};

pub(crate) static mut RUNTIME: *mut Runtime = std::ptr::null_mut();

pub struct Runtime {
    /// Available threads for the threadpool
    available_threads: Vec<usize>,
    /// All registered callbacks
    callback_queue: HashMap<usize, Box<dyn FnOnce(Option<String>) >>,
    // the unique id for callback funciton
    identity_token: usize,
    pending_events: usize,
    thread_pool: Vec<NodeThread>,    
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
        }
    }

    pub fn run(mut self) {
        let rt_ptr: *mut Runtime = &mut self;
        unsafe { RUNTIME = rt_ptr; }
        println!{"{:p}", rt_ptr};
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

    fn register_event_threadpool<T, U>(&mut self, task: T, cb: U)
    where 
        T: Fn() -> Option<String> + Send + 'static,
        U: FnOnce(Option<String>) + 'static 
    {
        let callback_id = self.generate_cb_identity();
        self.add_callback(callback_id, cb);

        let event = Task {
            task: Box::new(task),
            callback_id: callback_id,
            kind: ThreadPollTaskKind::FileRead, 
        };

        let available_thread = self.get_available_thread();
        self.thread_pool[available_thread]
            .sender
            .send(event)
            .expect("register work");
        self.pending_events += 1;
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