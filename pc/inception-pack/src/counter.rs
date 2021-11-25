pub struct U16Counter(u16);

impl U16Counter {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn next(&mut self) -> u16 {
        let result = self.0;
        self.0 += 1;
        result
    }
}
