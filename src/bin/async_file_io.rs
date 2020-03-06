use learn_async_with_rust::{runtime::Runtime, task::Fs};

fn async_file_io() {
    println!("First call to read test.txt");
    Fs::read(
        "/home/user1/rust_workplace/learn_async_with_rust/README.md",
        |result| {
            println!("{}", result.unwrap());
        },
    );

    Fs::read(
        "/home/user1/rust_workplace/learn_async_with_rust/README.md",
        |result| {
            println!("{}", result.unwrap());
        },
    );

    Fs::read(
        "/home/user1/rust_workplace/learn_async_with_rust/README.md",
        |result| {
            println!("{}", result.unwrap());
        },
    );

    Fs::read(
        "/home/user1/rust_workplace/learn_async_with_rust/README.md",
        |result| {
            println!("{}", result.unwrap());
        },
    );
}

fn main() {
    let runtime = Runtime::new();
    runtime.run(async_file_io);
}
