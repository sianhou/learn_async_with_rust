#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::{Event, TcpStream};

pub type Events = Vec<Event>;
pub type Token = usize;

const WRITEABLE: u8 = 0b0000_0001;
const READABLE: u8 = 0b0000_0010;

pub struct Interest(u8);
impl Interest {
    pub const WRITEABLE: Interest = Interest(WRITEABLE);
    pub const READABLE: Interest = Interest(READABLE);

    pub fn is_readable(&self) -> bool {
        self.0 & READABLE != 0
    }

    pub fn is_writeable(&self) -> bool {
        self.0 & WRITEABLE != 0
    }
}
