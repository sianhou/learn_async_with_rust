use async_with_callback::{
    runtime::Runtime,
    task::{Fibonacchi, Fs, Http},
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

    // `http_get_slow` let's us define a latency we want to simulate
    println!("Registering http get request to google.com");
    Http::http_get_slow("http//www.baidu.com", 500, |result| {
        let result = result.into_string().unwrap();
        print_content(result.trim());
    });
}

fn print_content(t: impl std::fmt::Display) {

    let content = format!("{}", t);
    let lines = content.lines().take(2);
    let main_cont: String = lines.map(|l| format!("{}\n", l)).collect();
    let opt_location = content.find("Location");

    let opt_location = opt_location.map(|loc| {
        content[loc..]
        .lines()
        .nth(0)
        .map(|l| format!("{}\n",l))
        .unwrap_or(String::new())
    });

    println!(
        "{}{}... [Note: Abbreviated for display] ...",
        main_cont,
        opt_location.unwrap_or(String::new())
    );

    println!("===== END CONTENT =====\n");
}

fn main() {
    let runtime = Runtime::new();
    runtime.run(thread_pool_event);
}
