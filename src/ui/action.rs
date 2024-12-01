use crate::storage::{Animation, AssetId, Project};
use egui::ViewportId;
use once_cell::unsync::OnceCell;
use std::{
    cell::{RefCell, RefMut},
    collections::VecDeque,
};

thread_local! {
    static ACTION_QUEUE: OnceCell<RefCell<VecDeque<Action>>> = const { OnceCell::new() };
}

pub enum Action {
    SetProject(AssetId<Project>),
    DeleteSelectedSceneInstance,
    CloneSelectedSceneInstance,
    InitGPU,
    SendPositions,
    ReloadShaderCode(AssetId<Animation>),
    CloseWindow(ViewportId),
}

impl Action {
    #[inline(always)]
    fn run_on_queue<T, F: FnOnce(RefMut<VecDeque<Action>>) -> T>(runner: F) -> T {
        ACTION_QUEUE.with(|queue| {
            let queue = queue.get_or_init(|| RefCell::new(VecDeque::new()));
            runner(queue.borrow_mut())
        })
    }

    pub fn enqueue(self) {
        Self::run_on_queue(move |mut queue| queue.push_back(self));
    }

    pub fn dequeue() -> Option<Self> {
        Self::run_on_queue(move |mut queue| queue.pop_front())
    }
}
