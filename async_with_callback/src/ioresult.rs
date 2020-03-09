#[derive(Debug)]
pub enum IOResult {
    Undefined,
    String(String),
    Int(usize),
}

impl IOResult {
    pub fn into_string(self) -> Option<String> {
        match self {
            IOResult::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn into_int(self) -> Option<usize> {
        match self {
            IOResult::Int(i) => Some(i),
            _ => None,
        }
    }
}