use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use anyhow::{anyhow, Context, Result};

use crate::asset::vmt::Vmt;
use crate::asset::vtf::Vtf;
use crate::file::FileLoader;
use crate::vpk::path::VpkPath;

pub mod vmt;
pub mod vtf;

pub trait Asset: Sized {
    fn from_data(loader: &AssetLoader, path: &VpkPath, data: Vec<u8>) -> Result<Rc<Self>>;
}

pub struct AssetLoader<'a> {
    material_loader: Rc<dyn FileLoader + 'a>,
    texture_loader: Rc<dyn FileLoader + 'a>,
    material_assets: RefCell<HashMap<VpkPath, Option<Rc<Vmt>>>>,
    texture_assets: RefCell<HashMap<VpkPath, Option<Rc<Vtf>>>>,
}

impl<'a> AssetLoader<'a> {
    pub fn new(
        material_loader: Rc<dyn FileLoader + 'a>,
        texture_loader: Rc<dyn FileLoader + 'a>,
    ) -> Self {
        Self {
            material_loader,
            texture_loader,
            material_assets: RefCell::new(HashMap::new()),
            texture_assets: RefCell::new(HashMap::new()),
        }
    }

    fn get<T: Asset>(
        &self,
        loader: &(dyn FileLoader + 'a),
        assets: &RefCell<HashMap<VpkPath, Option<Rc<T>>>>,
        path: &VpkPath,
    ) -> Result<Rc<T>> {
        if assets.borrow().contains_key(path) {
            Ok(Rc::clone(
                assets
                    .borrow()
                    .get(path)
                    .unwrap()
                    .as_ref()
                    .unwrap_or_else(|| panic!("Recursive load of asset {}", path)),
            ))
        } else {
            // Poison this entry to catch a recursive load of this asset.
            assets.borrow_mut().insert(path.clone(), None);
            drop(assets);

            match (|| {
                Ok(T::from_data(
                    self,
                    path,
                    loader
                        .load_file(path)?
                        .ok_or_else(|| anyhow!("file not found: {}", path))?,
                )
                .with_context(|| format!("Error creating asset from data for {}", path))?)
            })() {
                Ok(asset) => {
                    // Replace the poison entry with the loaded asset.
                    *assets.borrow_mut().get_mut(path).unwrap() = Some(Rc::clone(&asset));
                    Ok(asset)
                }
                Err(e) => {
                    // Remove the poison entry.
                    assets.borrow_mut().remove(path);
                    Err(e)
                }
            }
        }
    }

    pub fn get_material(&self, path: &VpkPath) -> Result<Rc<Vmt>> {
        self.get(&*self.material_loader, &self.material_assets, path)
    }

    pub fn get_texture(&self, path: &VpkPath) -> Result<Rc<Vtf>> {
        self.get(&*self.texture_loader, &self.texture_assets, path)
    }
}
