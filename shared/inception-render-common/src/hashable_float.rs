use core::cmp::Ordering;
use core::hash::{Hash, Hasher};

#[derive(Clone, Copy, Debug)]
pub struct HashableMat<const R: usize, const C: usize> {
    matrix: nalgebra_glm::TMat<f32, R, C>,
}

impl<const R: usize, const C: usize> Eq for HashableMat<R, C> {}

impl<const R: usize, const C: usize> Hash for HashableMat<R, C> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for r in 0..R {
            for c in 0..C {
                self.matrix[(r, c)].to_bits().hash(state);
            }
        }
    }
}

impl<const R: usize, const C: usize> Ord for HashableMat<R, C> {
    fn cmp(&self, other: &Self) -> Ordering {
        for r in 0..R {
            for c in 0..C {
                match self.matrix[(r, c)]
                    .to_bits()
                    .cmp(&other.matrix[(r, c)].to_bits())
                {
                    Ordering::Equal => (),
                    ordering => return ordering,
                }
            }
        }
        Ordering::Equal
    }
}

impl<const R: usize, const C: usize> PartialEq for HashableMat<R, C> {
    fn eq(&self, other: &Self) -> bool {
        for r in 0..R {
            for c in 0..C {
                if self.matrix[(r, c)].to_bits() != other.matrix[(r, c)].to_bits() {
                    return false;
                }
            }
        }
        true
    }
}

impl<const R: usize, const C: usize> PartialOrd for HashableMat<R, C> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub type HashableMat3 = HashableMat<3, 3>;
pub type HashableMat3x4 = HashableMat<3, 4>;
pub type HashableMat4 = HashableMat<4, 4>;
pub type HashableMat4x3 = HashableMat<4, 3>;

pub type HashableVec<const R: usize> = HashableMat<R, 1>;
