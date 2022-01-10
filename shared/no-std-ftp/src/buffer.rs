pub struct Buffer<const N: usize> {
    data: [u8; N],
    start: usize,
    end: usize,
}

impl<const N: usize> Buffer<N> {
    pub fn new() -> Self {
        Self {
            data: [0; N],
            start: 0,
            end: 0,
        }
    }

    pub fn try_fill_if_empty<E>(
        &mut self,
        f: impl FnOnce(&mut [u8]) -> Result<usize, E>,
    ) -> Result<(), E> {
        if (self.start..self.end).is_empty() {
            let n = f(&mut self.data)?;
            self.start = 0;
            self.end = n;
        }
        Ok(())
    }

    pub fn get(&self) -> &[u8] {
        &self.data[self.start..self.end]
    }

    pub fn consume(&mut self, size: usize) {
        self.start += size;
    }
}
