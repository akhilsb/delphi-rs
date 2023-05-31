use std::{time::SystemTime};

use async_recursion::async_recursion;
use types::{Replica, hash_cc::{CoinMsg, CTRBCMsg}};

use crate::node::{Context};
use crypto::hash::{Hash};

impl Context{
    #[async_recursion]
    pub async fn process_batch_wssecho(self: &mut Context,ctrbc:CTRBCMsg,master_root:Hash ,echo_sender:Replica){
        let now = SystemTime::now();
        let vss_state = &mut self.batchvss_state;
        let sec_origin = ctrbc.origin;
        // Highly unlikely that the node will get an echo before rbc_init message
        log::info!("Received ECHO message from {} for secret from {}",echo_sender,ctrbc.origin);
        // If RBC already terminated, do not consider this RBC
        if vss_state.terminated_secrets.contains(&sec_origin){
            log::info!("Terminated secretsharing of instance {} already, skipping this echo",sec_origin);
            return;
        }
        match vss_state.node_secrets.get(&sec_origin){
            None => {
                vss_state.add_echo(sec_origin, echo_sender, &ctrbc);
                return;
            }
            Some(_x) =>{}
        }
        let mp = vss_state.node_secrets.get(&sec_origin).unwrap().master_root;
        if mp != master_root || !ctrbc.verify_mr_proof(){
            log::error!("Merkle root of WSS Init from {} did not match Merkle root of ECHO from {}",sec_origin,self.myid);
            return;
        }
        vss_state.add_echo(sec_origin, echo_sender, &ctrbc);
        let hash_root = vss_state.echo_check(sec_origin, self.num_nodes, self.num_faults, self.batch_size);
        match hash_root {
            None => {
                return;
            },
            Some(vec_hash_root) => {
                let echos = vss_state.echos.get_mut(&sec_origin).unwrap();
                let shard = echos.get(&self.myid).unwrap();
                let ctrbc = CTRBCMsg::new(shard.0.clone(), shard.1.clone(), 0, sec_origin);
                vss_state.add_ready(sec_origin, self.myid, &ctrbc);
                self.broadcast(CoinMsg::BatchWSSReady(ctrbc.clone(), vec_hash_root.0, self.myid)).await;
                self.process_batchwssready( ctrbc.clone(), master_root, self.myid).await;
            }
        }
        self.add_benchmark(String::from("process_batch_wssecho"), now.elapsed().unwrap().as_nanos());
    }
}