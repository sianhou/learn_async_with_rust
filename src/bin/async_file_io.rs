use learn_async_with_rust::{
    ioresult::IOResult,
    runtime::Runtime,
    task::{Fibonacchi, Fs},
};

fn async_file_io() {
    Fs::read(
        "/home/user1/rust_workplace/learn_async_with_rust/README.md",
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
    runtime.run(async_file_io);
}
