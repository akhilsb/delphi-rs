
use async_trait::async_trait;
use futures_util::SinkExt;
use network::{Acknowledgement};
use tokio::sync::mpsc::UnboundedSender;
use types::{appxcon::WrapperMsg, SyncMsg};

#[derive(Debug, Clone)]
pub struct Handler {
    consensus_tx: UnboundedSender<WrapperMsg>,
}

impl Handler {
    pub fn new(consensus_tx: UnboundedSender<WrapperMsg>) -> Self {
        Self { consensus_tx }
    }
}

#[async_trait]
impl network::Handler<Acknowledgement, WrapperMsg>
    for Handler
{
    async fn dispatch(
        &self,
        msg: WrapperMsg,
        writer: &mut network::Writer<Acknowledgement>,
    ) {
        // Forward the message
        self.consensus_tx
            .send(msg)
            .expect("Failed to send message to the consensus channel");

        // Acknowledge
        writer
            .send(Acknowledgement::Pong)
            .await
            .expect("Failed to send an acknowledgement");
    }
}

#[derive(Debug, Clone)]
pub struct SyncHandler {
    consensus_tx: UnboundedSender<SyncMsg>,
}

impl SyncHandler {
    pub fn new(consensus_tx: UnboundedSender<SyncMsg>) -> Self {
        Self { consensus_tx }
    }
}

#[async_trait]
impl network::Handler<Acknowledgement, SyncMsg>
    for SyncHandler
{
    async fn dispatch(
        &self,
        msg: SyncMsg,
        writer: &mut network::Writer<Acknowledgement>,
    ) {
        // Forward the message
        self.consensus_tx
            .send(msg)
            .expect("Failed to send message to the consensus channel");

        // Acknowledge
        writer
            .send(Acknowledgement::Pong)
            .await
            .expect("Failed to send an acknowledgement");
    }
}
