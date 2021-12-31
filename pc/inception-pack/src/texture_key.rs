use std::borrow::Borrow;
use std::hash::{Hash, Hasher};

use source_reader::vpk::path::VpkPath;

#[cfg(test)]
use quickcheck::Arbitrary;

use crate::Plane;

pub trait TextureKey {
    fn as_borrowed_texture_key(&self) -> BorrowedTextureKey;
}

impl Eq for dyn TextureKey + '_ {}

impl Hash for dyn TextureKey + '_ {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_borrowed_texture_key().hash(state);
    }
}

impl PartialEq for dyn TextureKey + '_ {
    fn eq(&self, other: &Self) -> bool {
        self.as_borrowed_texture_key() == other.as_borrowed_texture_key()
    }
}

impl ToOwned for dyn TextureKey + '_ {
    type Owned = OwnedTextureKey;

    fn to_owned(&self) -> OwnedTextureKey {
        match self.as_borrowed_texture_key() {
            BorrowedTextureKey::EncodeAsIs { texture_path } => OwnedTextureKey::EncodeAsIs {
                texture_path: texture_path.to_owned(),
            },
            BorrowedTextureKey::Intensity { texture_path } => OwnedTextureKey::Intensity {
                texture_path: texture_path.to_owned(),
            },
            BorrowedTextureKey::AlphaToIntensity { texture_path } => {
                OwnedTextureKey::AlphaToIntensity {
                    texture_path: texture_path.to_owned(),
                }
            }
            BorrowedTextureKey::ComposeIntensityAlpha {
                intensity_texture_path,
                intensity_from_alpha,
                alpha_texture_path,
            } => OwnedTextureKey::ComposeIntensityAlpha {
                intensity_texture_path: intensity_texture_path.to_owned(),
                intensity_from_alpha,
                alpha_texture_path: alpha_texture_path.to_owned(),
            },
            BorrowedTextureKey::BakeOrientedEnvmap {
                texture_path,
                plane,
            } => OwnedTextureKey::BakeOrientedEnvmap {
                texture_path: texture_path.to_owned(),
                plane: *plane,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum OwnedTextureKey {
    EncodeAsIs {
        texture_path: VpkPath,
    },
    Intensity {
        texture_path: VpkPath,
    },
    AlphaToIntensity {
        texture_path: VpkPath,
    },
    ComposeIntensityAlpha {
        intensity_texture_path: VpkPath,
        intensity_from_alpha: bool,
        alpha_texture_path: VpkPath,
    },
    BakeOrientedEnvmap {
        texture_path: VpkPath,
        plane: Plane,
    },
}

#[cfg(test)]
impl Arbitrary for OwnedTextureKey {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        match u8::arbitrary(g) % 5 {
            0 => Self::EncodeAsIs {
                texture_path: VpkPath::arbitrary(g),
            },
            1 => Self::Intensity {
                texture_path: VpkPath::arbitrary(g),
            },
            2 => Self::AlphaToIntensity {
                texture_path: VpkPath::arbitrary(g),
            },
            3 => Self::ComposeIntensityAlpha {
                intensity_texture_path: VpkPath::arbitrary(g),
                intensity_from_alpha: bool::arbitrary(g),
                alpha_texture_path: VpkPath::arbitrary(g),
            },
            4 => Self::BakeOrientedEnvmap {
                texture_path: VpkPath::arbitrary(g),
                plane: Plane::arbitrary(g),
            },
            _ => unreachable!(),
        }
    }
}

impl TextureKey for OwnedTextureKey {
    fn as_borrowed_texture_key(&self) -> BorrowedTextureKey {
        match self {
            Self::EncodeAsIs { texture_path } => BorrowedTextureKey::EncodeAsIs { texture_path },
            Self::Intensity { texture_path } => BorrowedTextureKey::Intensity { texture_path },
            Self::AlphaToIntensity { texture_path } => {
                BorrowedTextureKey::AlphaToIntensity { texture_path }
            }
            Self::ComposeIntensityAlpha {
                intensity_texture_path,
                intensity_from_alpha,
                alpha_texture_path,
            } => BorrowedTextureKey::ComposeIntensityAlpha {
                intensity_texture_path,
                intensity_from_alpha: *intensity_from_alpha,
                alpha_texture_path,
            },
            Self::BakeOrientedEnvmap {
                texture_path,
                plane,
            } => BorrowedTextureKey::BakeOrientedEnvmap {
                texture_path,
                plane,
            },
        }
    }
}

impl<'a> Borrow<dyn TextureKey + 'a> for OwnedTextureKey {
    fn borrow(&self) -> &(dyn TextureKey + 'a) {
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BorrowedTextureKey<'a> {
    EncodeAsIs {
        texture_path: &'a VpkPath,
    },
    Intensity {
        texture_path: &'a VpkPath,
    },
    AlphaToIntensity {
        texture_path: &'a VpkPath,
    },
    ComposeIntensityAlpha {
        intensity_texture_path: &'a VpkPath,
        intensity_from_alpha: bool,
        alpha_texture_path: &'a VpkPath,
    },
    BakeOrientedEnvmap {
        texture_path: &'a VpkPath,
        plane: &'a Plane,
    },
}

impl<'a> TextureKey for BorrowedTextureKey<'a> {
    fn as_borrowed_texture_key(&self) -> BorrowedTextureKey {
        *self
    }
}

impl<'a> Borrow<dyn TextureKey + 'a> for BorrowedTextureKey<'a> {
    fn borrow(&self) -> &(dyn TextureKey + 'a) {
        self
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Borrow;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    use super::{OwnedTextureKey, TextureKey};

    #[quickcheck]
    fn borrowing_preserves_hash(owned: OwnedTextureKey) -> bool {
        let owned_hash = {
            let mut s = DefaultHasher::new();
            owned.hash(&mut s);
            s.finish()
        };
        let borrowed_hash = {
            let mut s = DefaultHasher::new();
            <OwnedTextureKey as Borrow<dyn TextureKey>>::borrow(&owned).hash(&mut s);
            s.finish()
        };
        owned_hash == borrowed_hash
    }

    #[quickcheck]
    fn to_owned_round_trips(owned: OwnedTextureKey) -> bool {
        owned == <OwnedTextureKey as Borrow<dyn TextureKey>>::borrow(&owned).to_owned()
    }
}
