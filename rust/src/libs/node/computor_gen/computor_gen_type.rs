pub enum ComputorMsg {
    Tick,
    Start(String),
    Stop,
    SetEmissionAddress(String),
}

#[derive(Clone)]
pub struct ComputorState {
    pub enabled: bool,
    pub ctype: Option<String>, // could be "trainer" or None
}

impl ComputorState {
    pub fn new() -> Self {
        Self {
            enabled: false,
            ctype: None,
        }
    }
}
