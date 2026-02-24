use crate::storage::{asset::AssetTrait, collection::Collection};
use crossbeam_channel::{Receiver, Sender};
use egui::mutex::Mutex;
use once_cell::sync::Lazy;
use typemap::ShareCloneMap;

static SENDERS: Lazy<Mutex<Vec<Sender<ShareCloneMap>>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub struct Collections {
    receiver: Receiver<ShareCloneMap>,
    current: ShareCloneMap,
}

impl Default for Collections {
    fn default() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        SENDERS.lock().push(sender);

        Self {
            receiver,
            current: ShareCloneMap::custom(),
        }
    }
}

impl Collections {
    pub fn update(&mut self) {
        while let Ok(new_collections) = self.receiver.try_recv() {
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
        // sends to myself, but we ignore that for now
        SENDERS
            .lock()
            .retain(|sender| sender.send(self.current.clone()).is_ok());
    }
}
