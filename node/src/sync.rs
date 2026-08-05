pub trait ChainSync {
    fn sync(&mut self);
}

pub struct SyncManager;

impl ChainSync for SyncManager {
    fn sync(&mut self) {
        println!("Syncing chain... (Stub)");
    }
}
