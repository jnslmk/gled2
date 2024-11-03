use super::AssetTrait;
use crate::storage::{collection::Collection, Action, State, STATE};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct AssetId<T> {
    pub id: Uuid,
    #[serde(skip)]
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Clone> Copy for AssetId<T> {}

impl<T> Hash for AssetId<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T: AssetTrait> AssetId<T> {
    pub fn new() -> Self {
        Self::from_uuid(Uuid::new_v4())
    }

    pub fn from_uuid(id: Uuid) -> Self {
        Self {
            id,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn delete(self) {
        log::info!("Deleting palette from cache: {:?}", self);

        std::thread::spawn(move || {
            let state: &mut State = &mut STATE.lock();
            if let State::Opened { collections, .. } = state {
                if let Some(collection) = collections.get_mut::<Collection<T>>() {
                    collection.delete_asset(self);
                }
            }

            Action::DeleteAsset {
                uuid: self.id,
                dir_name: T::DIR_NAME,
            }
            .send();
        });
    }
}
