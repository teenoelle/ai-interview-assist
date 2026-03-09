// VAD is now integrated into RingBuffer, this module kept for future use

pub struct EnergyVad;

impl EnergyVad {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EnergyVad {
    fn default() -> Self {
        Self::new()
    }
}
