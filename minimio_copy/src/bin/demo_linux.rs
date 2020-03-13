mod ffi {
    #[link(name = "c")]
    extern "C" {
        pub fn epoll_create(size: i32) -> i32;
    }
}

fn main() {
    
}