pub enum PollEvent {
    /// An event from the `threadpool` with a tuple containing the `thread id`,
    /// the `callback_id` and the data which the we expect to process in our
    /// callback
    Threadpool((usize, usize, Option<String>)),
}