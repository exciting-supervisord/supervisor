
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum AutoRestart {
    Unexpected,
    Always,
    Never,
}