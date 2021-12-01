use num_traits::PrimInt;

pub struct Counter<T>(T);

impl<T: PrimInt> Counter<T> {
    pub fn new() -> Self {
        Self(T::zero())
    }

    pub fn next(&mut self) -> T {
        let result = self.0;
        self.0 = self.0.checked_add(&T::one()).unwrap();
        result
    }
}
