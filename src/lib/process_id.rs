#[derive(PartialEq, Eq, Hash, Clone)]
pub struct ProcessId {
    pub name: String,
    pub index: u32,
}

impl ProcessId {
    pub fn new(name: String, index: u32) -> Self {
        ProcessId { name, index }
    }
}
