use async_with_callback::{
    runtime::Runtime,
    task::{Fibonacchi, Fs},
};

fn thread_pool_event() {
    Fs::read(
        "./README.md",
        |result| {
            println!("{}", result.into_string().unwrap());
        },
    );

    Fibonacchi::cal(5, |result| {
        println!("{}", result.into_int().unwrap());
    });

    Fibonacchi::cal(10, |result| {
        println!("{}", result.into_int().unwrap());
    });

    Fibonacchi::cal(15, |result| {
        println!("{}", result.into_int().unwrap());
    });

    Fibonacchi::cal(20, |result| {
        println!("{}", result.into_int().unwrap());
    });
}

fn main() {
    let runtime = Runtime::new();
    runtime.run(thread_pool_event);
}
