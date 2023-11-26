#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct ProcessId {
    pub name: String,
    pub seq: u32,
}

impl ProcessId {
    pub fn new(name: String, seq: u32) -> Self {
        ProcessId { name, seq }
    }
}

impl std::fmt::Display for ProcessId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.name, self.seq)
    }
}
