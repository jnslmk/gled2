use crate::storage::{asset::AssetTrait, collection::Collection};
use egui::mutex::Mutex;
use kanal::{Receiver, Sender, bounded};
use once_cell::sync::Lazy;
use std::{collections::HashMap, sync::atomic::AtomicUsize};
use typemap::ShareCloneMap;

static CURRENT: Lazy<Mutex<ShareCloneMap>> = Lazy::new(|| Mutex::new(ShareCloneMap::custom()));
static COLLECTION_ID: AtomicUsize = AtomicUsize::new(0);
static SENDERS: Lazy<Mutex<HashMap<usize, Sender<ShareCloneMap>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub struct Collections {
    id: usize,
    receiver: Receiver<ShareCloneMap>,
    current: ShareCloneMap,
}

impl Default for Collections {
    fn default() -> Self {
        let (sender, receiver) = bounded(8);
        let id = COLLECTION_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        SENDERS.lock().insert(id, sender);

        Self {
            id,
            receiver,
            current: CURRENT.lock().clone(),
        }
    }
}

impl Drop for Collections {
    fn drop(&mut self) {
        SENDERS.lock().remove(&self.id);
    }
}

impl Collections {
    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn update(&mut self) {
        while let Ok(Some(new_collections)) = self.receiver.try_recv() {
            self.current = new_collections;
        }
    }

    pub fn insert<T>(&mut self, val: Collection<T>)
    where
        T: AssetTrait,
    {
        self.current.insert::<Collection<T>>(val);
    }

    pub fn get<T>(&self) -> Option<&Collection<T>>
    where
        T: AssetTrait,
    {
        self.current.get::<Collection<T>>()
    }

    pub fn get_mut<T>(&mut self) -> &mut Collection<T>
    where
        T: AssetTrait,
    {
        self.current
            .entry::<Collection<T>>()
            .or_insert_with(Default::default)
    }

    pub fn save(&self) {
        for (id, sender) in SENDERS.lock().iter() {
            if id == &self.id {
                continue;
            }

            sender
                .send(self.current.clone())
                .expect("Could not send collections");
        }

        CURRENT.lock().clone_from(&self.current);
    }
}
