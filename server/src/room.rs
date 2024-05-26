use crate::gen_id;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct RoomInstance {}

impl RoomInstance {
	pub fn new() -> Self {
		Self {}
	}
}

pub type RoomInstanceMutex = Arc<Mutex<RoomInstance>>;
gen_id!(RoomVec, RoomInstanceMutex, RoomCell, RoomHandle);
