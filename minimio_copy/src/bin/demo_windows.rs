pub mod demo_windows {

    #![allow(non_camel_case_types)]

    use std::io::{self, Write};
    use std::net::TcpStream;
    use std::os::windows::io::AsRawSocket;
    use std::ptr;

    pub fn test() {
        let mut event_counter = 0;

        let queue = unsafe {
            ffi::CreateIoCompletionPort(ffi::INVALID_HANDLE_VALUE, 0, ptr::null_mut(), 0)
        };
        if (queue as *mut usize).is_null() {
            panic!(std::io::Error::last_os_error());
        }

        let mut streams = vec![];

        #[derive(Debug)]
        #[repr(C)]
        struct Operation {
            wsaoverlapped: ffi::WSAOVERLAPPED,
            context: usize,
        };

        let mut ops: Vec<Operation> = Vec::with_capacity(5);
        let mut buffers: Vec<Vec<u8>> = Vec::with_capacity(5);

        for i in 0..5 {
            let addr = "slowwly.robertomurray.co.uk:80";
            let mut stream = TcpStream::connect(addr).unwrap();

            let delay = (4 - i) * 1000;
            let request = format!(
                "GET /delay/{}/url/http://www.baidu.com HTTP/1.1\r\n\
                 Host: slowwly.robertomurray.co.uk\r\n\
                 Connection: close\r\n\
                 \r\n",
                delay
            );
            stream.write_all(request.as_bytes()).unwrap();

            stream.set_nonblocking(true).unwrap();

            let res = unsafe {
                ffi::CreateIoCompletionPort(
                    stream.as_raw_socket() as isize,
                    queue,
                    ptr::null_mut(),
                    0,
                )
            };

            if (res as *mut usize).is_null() {
                panic!(std::io::Error::last_os_error());
            }

            let event = ffi::WSAOVERLAPPED::zeroed();
            let op = Operation {
                wsaoverlapped: event,
                context: i,
            };
            ops.push(op);
            let op = ops.last_mut().unwrap();
            let op_ptr: *mut Operation = op;

            let buffer: Vec<u8> = Vec::with_capacity(1024);
            buffers.push(buffer);
            let buffer = buffers.last_mut().unwrap();

            let wsabuf = &mut [ffi::WSABUF::new(1024, buffer.as_mut_ptr())];
            let mut flags = 0;
            let res = unsafe {
                ffi::WSARecv(
                    stream.as_raw_socket(),
                    wsabuf.as_mut_ptr(),
                    1,
                    ptr::null_mut(),
                    &mut flags,
                    op_ptr as *mut ffi::WSAOVERLAPPED,
                    ptr::null_mut(),
                )
            };

            if res != 0 {
                let err = unsafe { ffi::WSAGetLastError() };
                if err == ffi::WSA_IO_PENDING {
                    ()
                } else {
                    panic!(std::io::Error::last_os_error());
                }
            } else {
                ()
            };

            streams.push(stream);
            event_counter += 1;
        }

        while event_counter > 0 {
            let mut events: Vec<ffi::OVERLAPPED_ENTRY> = Vec::with_capacity(10);
            let mut entries_removed: u32 = 0;
            let res = unsafe {
                ffi::GetQueuedCompletionStatusEx(
                    queue,
                    events.as_mut_ptr(),
                    events.capacity() as u32,
                    &mut entries_removed,
                    ffi::INFINITE,
                    false,
                )
            };

            if res == 0 {
                panic!(io::Error::last_os_error());
            };

            unsafe { events.set_len(res as usize) };

            for event in events {
                let operation = unsafe { &*(event.lp_overlapped as *const Operation) };
                println!("RECIEVED: {}", operation.context);
                event_counter -= 1;
            }
        }

        let res = unsafe { ffi::CloseHandle(queue) };
        if res == 0 {
            panic!(io::Error::last_os_error());
        };
        println!("FINISHED");
    }

    mod ffi {
        #[link(name = "Kernel32")]
        extern "stdcall" {
            pub fn CreateIoCompletionPort(
                filehandle: HANDLE,
                existing_completionport: HANDLE,
                completion_key: ULONG_PTR,
                number_of_concurrent_threads: DWORD,
            ) -> HANDLE;

            pub fn WSARecv(
                s: RawSocket,
                lpBuffers: LPWSABUF,
                dwBufferCount: DWORD,
                lpNumberOfBytesRecvd: LPDWORD,
                lpFlags: LPDWORD,
                lpOverlapped: LPWSAOVERLAPPED,
                lpCompletionRoutine: LPWSAOVERLAPPED_COMPLETION_ROUTINE,
            ) -> i32;

            pub fn GetQueuedCompletionStatusEx(
                CompletionPort: HANDLE,
                lpCompletionPortEntries: *mut OVERLAPPED_ENTRY,
                ulCount: ULONG,
                ulNumEntriesRemoved: PULONG,
                dmMilliseconds: DWORD,
                fAlterable: BOOL,
            ) -> i32;

            pub fn CloseHandle(hObject: HANDLE) -> i32;

            pub fn WSAGetLastError() -> i32;
        }

        use std::os::windows::io::RawSocket;
        use std::ptr;

        #[repr(C)]
        #[derive(Clone, Debug)]
        pub struct WSABUF {
            len: u32,
            buf: *mut u8,
        }

        impl WSABUF {
            pub fn new(len: u32, buf: *mut u8) -> Self {
                WSABUF { len, buf }
            }
        }

        #[repr(C)]
        #[derive(Debug, Clone)]
        pub struct OVERLAPPED_ENTRY {
            lp_completion_key: *mut usize,
            pub lp_overlapped: *mut WSAOVERLAPPED,
            internal: usize,
            bytes_transferred: u32,
        }

        impl OVERLAPPED_ENTRY {
            pub fn zeroed() -> Self {
                OVERLAPPED_ENTRY {
                    lp_completion_key: ptr::null_mut(),
                    lp_overlapped: ptr::null_mut(),
                    internal: 0,
                    bytes_transferred: 0,
                }
            }
        }

        #[repr(C)]
        #[derive(Debug)]
        pub struct WSAOVERLAPPED {
            internal: ULONG_PTR,
            internal_high: ULONG_PTR,
            offset: DWORD,
            offset_high: DWORD,
            h_event: HANDLE,
        }

        impl WSAOVERLAPPED {
            pub fn zeroed() -> Self {
                WSAOVERLAPPED {
                    internal: ptr::null_mut(),
                    internal_high: ptr::null_mut(),
                    offset: 0,
                    offset_high: 0,
                    h_event: 0,
                }
            }
        }

        pub type HANDLE = isize;
        pub type BOOL = bool;
        pub type DWORD = u32;
        pub type ULONG = u32;
        pub type PULONG = *mut ULONG;
        pub type ULONG_PTR = *mut usize;
        pub type PULONG_PTR = *mut ULONG_PTR;
        pub type LPDWORD = *mut DWORD;
        pub type LPWSABUF = *mut WSABUF;
        pub type LPWSAOVERLAPPED = *mut WSAOVERLAPPED;
        pub type LPWSAOVERLAPPED_COMPLETION_ROUTINE = *const extern "C" fn();

        // https://referencesource.microsoft.com/#System.Runtime.Remoting/channels/ipc/win32namedpipes.cs,edc09ced20442fea,references
        // read this! https://devblogs.microsoft.com/oldnewthing/20040302-00/?p=40443
        /// Defined in `win32.h` which you can find on your windows system
        pub const INVALID_HANDLE_VALUE: HANDLE = -1;

        // https://docs.microsoft.com/en-us/windows/win32/winsock/windows-sockets-error-codes-2
        pub const WSA_IO_PENDING: i32 = 997;

        // This can also be written as `4294967295` if you look at sources on the internet.
        // Interpreted as an i32 the value is -1
        // see for yourself: https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=4b93de7d7eb43fa9cd7f5b60933d8935
        pub const INFINITE: u32 = 0xFFFFFFFF;
    }
}

fn main() {
    #[cfg(target_os = "windows")]
    demo_windows::test();

    #[cfg(not(target_os = "windows"))]
    println!("Target_os != windows.");
}
