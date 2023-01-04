#[derive(PartialEq, Eq, Hash, Clone)]
pub struct ProcessId {
    pub name: String,
    pub seq: u32,
}

impl ProcessId {
    pub fn new(name: String, seq: u32) -> Self {
        ProcessId { name, seq }
    }
}
