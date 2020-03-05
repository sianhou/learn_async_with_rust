use learn_async_with_rust::runtime::Runtime;

fn async_fn() {
    
}

fn main() {
    let runtime = Runtime::new();
    runtime.run();

    learn_async_with_rust::task::Fs::read();
}