use std::sync::mpsc::channel;
use std::{ thread, collections::HashMap };

use crate::nodethread::NodeThread;
use crate::pollevent::PollEvent;
use crate::task::Task;

pub struct Runtime {
    /// All registered callbacks
    callback_queue: HashMap<usize, Box<dyn FnOnce(Option<String>)>>,
    // the unique id for callback funciton
    identity_token: usize,
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
            callback_queue: HashMap::new(),
            identity_token: 0,
            thread_pool: threads,
        }
    }

    fn generate_identity(&mut self) -> usize {
        self.identity_token = self.identity_token.wrapping_add(1);
        self.identity_token
    }

    pub fn register_event_threadpool(&mut self) -> usize {
        loop {
            let ident = self.generate_identity();
            if !self.callback_queue.contains_key(&ident) {
                break ident;
            }
        }
    }
}

#[test]
fn test_register_event_threadpool() {
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