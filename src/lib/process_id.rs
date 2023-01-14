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

impl std::fmt::Display for ProcessId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.name, self.seq)
    }
}

pub trait ToProcessIds {
    fn to_process_ids(&self) -> Vec<ProcessId>
    where
        Self: IntoIterator<Item = String>;
}

impl ToProcessIds for Vec<String> {
    fn to_process_ids(&self) -> Vec<ProcessId>
    where
        Self: IntoIterator<Item = String>,
    {
        self.iter()
            .map(|x| {
                let (name, seq) = x.split_once(":").expect("return Invalid argument");
                ProcessId::new(name.to_owned(), seq.parse::<u32>().expect("parse fail"))
            })
            .collect::<Vec<ProcessId>>()
    }
}
