use serde::{Serialize, Deserialize};

use crate::{WireReady, Replica, Val};

#[derive(Debug,Serialize,Deserialize,Clone)]
pub enum SyncState{
    ALIVE,
    START,
    StartRecon,
    STARTED,
    CompletedSharing,
    COMPLETED,
    CompletedRecon,
    STOP,
    STOPPED
}

#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct SyncMsg{
    pub sender:Replica,
    pub state:SyncState,
    pub value: Val
}

impl WireReady for SyncMsg{
    fn from_bytes(bytes: &[u8]) -> Self {
        let c:Self = bincode::deserialize(bytes)
            .expect("failed to decode the protocol message");
        c.init()
    }

    fn to_bytes(&self) -> Vec<u8> {
        let bytes = bincode::serialize(self).expect("Failed to serialize client message");
        bytes
    }

    fn init(self) -> Self {
        match self {
            _x=>_x
        }
    }
}