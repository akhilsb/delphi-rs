use std::collections::{HashMap, BTreeMap};

use async_recursion::async_recursion;
use crypto_blstrs::{threshold_sig::{PartialBlstrsSignature, BlstrsSignature}, crypto::threshold_sig::{SecretKey, PublicKey, CombinableSignature, Signature}};
use types::{Round, appxcon::ProtMsg, Replica};

use super::Context;

impl Context {
    #[async_recursion]
    pub async fn elect_leader(&mut self, round:Round){
        if !self.leader_election_state.contains_key(&round){
            let log_n = (self.num_nodes as f64).log2().ceil() as u32;
            let mut rand_map:HashMap<usize, Vec<PartialBlstrsSignature>> = HashMap::default();
            for index in 0..log_n{
                let mut beacon_msg = self.sign_msg.clone();
                beacon_msg.push_str(round.to_string().as_str());
                beacon_msg.push_str(index.to_string().as_str());
                let dst = "Test";
                let psig = self.secret_key.sign(&beacon_msg, &dst);
                // Send outgoing messages
                let mut partial_sigs:Vec<PartialBlstrsSignature> = Vec::new();
                partial_sigs.push(psig.clone());
                rand_map.insert(index as usize, partial_sigs);
                let sig_data = bincode::serialize(&psig).expect("Serialization error");
                let prot_msg = ProtMsg::LeaderCoin(round, index as usize, sig_data, self.myid);
                log::info!("Sending signature on string {} from node {} on round {} and index {}",beacon_msg,self.myid,round,index);
                self.broadcast(prot_msg).await;
            }
            let bitmap = BTreeMap::default();
            self.leader_election_state.insert(round, (rand_map,bitmap,None));
        }
    }

    pub async fn handle_incoming_leader_coin(&mut self,round:Round,index:usize,psig:Vec<u8>,share_sender:Replica){
        if !self.leader_election_state.contains_key(&round){
            self.elect_leader(round).await;
        }
        let psig_deser:PartialBlstrsSignature = bincode::deserialize(psig.as_slice()).expect("Deserialization error");
        // Add signature to state
        let sig_state = self.leader_election_state.get_mut(&round).unwrap();
        let sig_vec = sig_state.0.get_mut(&index).unwrap();
        if sig_vec.len() > ((self.num_faults + 1) as usize){
            return;
        }
        else{
            let pkey = self.tpubkey_share.get(&(share_sender+1)).unwrap();
            let mut beacon_msg = self.sign_msg.clone();
            beacon_msg.push_str(round.to_string().as_str());
            beacon_msg.push_str(index.to_string().as_str());
            let dst = "Test";
            if pkey.verify(&psig_deser, &beacon_msg, &dst){
                log::info!("Signature verification successful, adding sig to map");
                sig_vec.push(psig_deser);
            }
            else {
                log::error!("Signature verification unsuccessful");
                return;
            }
            if sig_vec.len() == (self.num_faults+1) as usize{
                log::info!("Aggregating signatures for round {}",round);
                let sig = BlstrsSignature::combine((self.num_faults+1) as usize, sig_vec.clone()).expect("Unable to combine threshold sigs");
                let result =  sig.rand_coin(1,2).unwrap();
                if result{
                    sig_state.1.insert(index, 1);
                }
                else {
                    sig_state.1.insert(index, 0);
                }
                let log_n = (self.num_nodes as f64).log2().ceil() as usize;
                if sig_state.1.len() == log_n{
                    log::info!("Sig state :{:?}",sig_state.1);
                    let mut power_2 = 1 as usize;
                    let mut sum = 0 as usize;
                    for (_in,bit) in sig_state.1.clone().into_iter(){
                        sum += power_2*(bit as usize);
                        power_2 = power_2*2;
                    }
                    log::info!("Elected leader {}",sum);
                    sig_state.2 = Some(sum);
                    if self.witnesses.contains(&sum){
                        // Input 1 to Binary BA instance
                        self.start_baa(round,0, 2, false).await;
                        log::info!("Witness found, inputting 1 to binary BA instance");
                    }
                    else {
                        // Input 0 to Binary BA instance
                        self.start_baa(round,0, 0, false).await;
                        log::info!("Witness not found, inputting 0 to binary BA instance");
                    }
                }
            }
        }
    }   
}