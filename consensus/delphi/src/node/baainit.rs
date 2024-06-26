use types::{Replica, SyncMsg, SyncState, appxcon::{ProtMsg, DelphiMsg}, Round, Val, Point, Lev};

use crate::node::{Context};

impl Context{
    /**
     * This function contains code to process an ECHO1 message from other nodes. 
     * An ECHO1 message is sent as part of the Binary Approximate Agreement (BinAA) protocol. 
     * Please refer to Algorithm 1 in our paper for the protocol's description. 
     */
    #[async_recursion::async_recursion]
    pub async fn process_baa_echo(self: &mut Context, msg:DelphiMsg, echo_sender:Replica, round:Round){
        if self.round > round{
            return;
        }
        log::info!("Received ECHO1 message from node {} with content {:?} for round {}",echo_sender,msg,round);
        // We coalesce an ECHO1 message. AN ECHO1 message can contain ECHO1s from various BinAA instances from 
        // various checkpoints at various levels. In order to save network bandwidth, we `coalesce` messages
        // from multiple checkpoints at each level into a single message. Refer to our paper for more details. 
        let mut levels_vals = msg.vals.clone();
        let coalesced_msg = msg.coalesce.clone();
        if coalesced_msg.1 < self.total_levels{
            let mut expanded_msg = Self::expand(coalesced_msg,self.rho,self.max_input,self.exponent).await;
            levels_vals.append(&mut expanded_msg);
        }
        let mut echo1msgs = Vec::new();
        let mut echo2msgs = Vec::new();
        for (level,interval_vals) in levels_vals.into_iter(){
            if self.round_state.contains_key(&level){
                let level_state = self.round_state.get_mut(&level).unwrap();
                // Each level has its own state. It comprises of checkpoints from [Integer.MIN_VALUE,Integer.MAX_VALUE] separated by \rho distance.
                // We run an approximate agreement instance for each checkpoint in the level. 
                let (echo1,echo2) = level_state.add_echo(interval_vals, round, echo_sender);
                if !echo1.is_empty(){
                    echo1msgs.push((level,echo1));
                }
                if !echo2.is_empty(){
                    echo2msgs.push((level,echo2));
                }
            }
        }
        let mut terminated = true;
        for level_state in self.round_state.iter(){
            terminated = terminated && level_state.1.terminated_round(round);
        }
        // If any checkpoint receives more than t+1 ECHO1s on a value, this node amplifies that ECHO1 and sends it to everyone.
        if echo1msgs.len() > 0{
            let (coalesced_msgs,echo_msgs) = Self::coalesce(echo1msgs, self.total_levels, self.max_input, round);
            let msg = DelphiMsg{origin:self.myid,round:round,vals: echo_msgs,coalesce:coalesced_msgs};
            self.broadcast(ProtMsg::BinaryAAEcho(msg.clone(), self.myid, round)).await;
        }
        // If any checkpoint receives more than 2t+1 ECHO1s on a value v, this node amplifies that ECHO1 and sends it to everyone.
        if echo2msgs.len() > 0{
            let (coalesced_msgs,echo2_msgs) = Self::coalesce(echo2msgs, self.total_levels, self.max_input, round);
            let msg = DelphiMsg{origin:self.myid,round:round,vals: echo2_msgs,coalesce:coalesced_msgs};
            self.broadcast(ProtMsg::BinaryAAEcho2(msg.clone(), self.myid, round)).await;
            self.process_baa_echo2( msg, self.myid, round).await;
        }
        if terminated {
            log::info!("Terminated round {}, starting round {}",round,round+1);
            self.start_baa(round+1).await;
            return;
        }
    }

    /**
     * This function contains code to process an ECHO2 message from other nodes. 
     * An ECHO2 message is sent as part of the Binary Approximate Agreement (BinAA) protocol. 
     * Please refer to Algorithm 1 in our paper for the protocol's description. 
     */
    pub async fn process_baa_echo2(self: &mut Context, msg: DelphiMsg, echo2_sender:Replica, round:Round){
        // discard older messages
        if self.round > round{
            return;
        }
        log::info!("Received ECHO2 message from node {} with content {:?} for round {}",echo2_sender,msg,round);
        if self.round > round{
            return;
        }
        // We coalesce an ECHO2 message. An ECHO2 message can contain ECHO2s from various BinAA instances from 
        // various checkpoints at various levels. In order to save network bandwidth, we `coalesce` messages
        // from multiple checkpoints at each level into a single message. Refer to our paper for more details.
        let mut levels_vals = msg.vals.clone();
        let coalesced_msg = msg.coalesce.clone();
        if coalesced_msg.1 < self.total_levels{
            let mut expanded_msg = Self::expand(coalesced_msg,self.rho,self.max_input,self.exponent).await;
            levels_vals.append(&mut expanded_msg);
        }
        for (level,interval_vals) in levels_vals.into_iter(){
            if self.round_state.contains_key(&level){
                let level_state = self.round_state.get_mut(&level).unwrap();
                level_state.add_echo2(interval_vals, round, echo2_sender);
            }
        }
        let mut terminated = true;
        for level_state in self.round_state.iter(){
            terminated = terminated && level_state.1.terminated_round(round);
        }
        if terminated {
            log::info!("Terminated round {}, starting round {}",round,round+1);
            self.start_baa(round+1).await;
            return;
        }
    }

    /**
     * This function starts a new round r of Binary Approximate Agreement.
     * We start a new round r only after all approximate agreement instances representing all checkpoints at all levels terminate round r-1.
     */
    #[async_recursion::async_recursion]
    pub async fn start_baa(self: &mut Context, round:Round){
        self.round = round;
        // If maximum number of rounds are run, terminate protocol by sending a Completed message to the syncer. 
        if self.round > self.total_rounds_bin{
            let final_val = self.aggregate();
            let cancel_handler = self.sync_send.send(0, SyncMsg { sender: self.myid, state: SyncState::CompletedSharing, value: final_val }).await;
            self.add_cancel_handler(cancel_handler);
            return;
        }
        let mut echo_msgs = Vec::new();
        for level in 0..self.total_levels{
            echo_msgs.push((level,self.round_state.get_mut(&level).unwrap().start_round(round, self.myid, self.input,self.max_input)));
        }
        let (coalesce_msg,echo_msgs) = Self::coalesce(echo_msgs, self.total_levels, self.max_input, round);
        let delphi_msg = DelphiMsg{
            origin: self.myid,
            round: round,
            vals: echo_msgs,
            coalesce:coalesce_msg
        };
        let prot_msg = ProtMsg::BinaryAAEcho(delphi_msg.clone(), self.myid,round);
        self.broadcast(prot_msg.clone()).await;
        self.process_baa_echo(delphi_msg, self.myid, round).await;
        log::info!("Broadcasted message {:?}",prot_msg);
    }

    /**
     * This function expands a coalesced message. Check the coalesce subroutine. This coalescing and expanding saves network bandwidth. 
     */
    pub async fn expand(coalesced_msg:(Point,Lev,Lev,Round),rho:Val,max_input:Val, exponent:f32)->Vec<(Lev, Vec<(Point, Point, Val)>)>{
        let mut levels_vals = Vec::new();
        for level in coalesced_msg.1..coalesced_msg.2+1{
            let mut level_vals = Vec::new();
            let sep = rho*((exponent.powf(level as f32).ceil()) as Val);
            let interval_start = ((coalesced_msg.0/sep)-1)*(sep);
            let interval_end = ((coalesced_msg.0/sep)+2)*(sep);

            let int_s = (Val::MIN,interval_start,0 as Val);
            let int_m = (interval_start,interval_end,max_input);
            let int_e = (interval_end,Val::MAX,0 as Val);

            level_vals.push(int_s);
            level_vals.push(int_m);
            level_vals.push(int_e);

            levels_vals.push((level,level_vals));
        }
        return levels_vals;
    }

    /**
     * Coalesce subroutine: If all levels $l>=l_t$ has only one checkpoint with a positive weight, we send a coalesce message. 
     * This message compresses messages from levels $l>=l_t$ into a single message (\mu,l_t,l_max,round).
     * This signifies all levels >l_t have only one non-zero weight. This non-zero weight is for the interval containing \mu.  
     */
    pub fn coalesce(echos: Vec<(Lev,Vec<(Point,Point,Val)>)>,max_levels:Lev,max_input:Val,round:Round)->((Point,Lev,Lev,Round),Vec<(Lev,Vec<(Point,Point,Val)>)>){
        let mut level_thresh = max_levels+1;
        let mut coalesce_val = Val::MIN;
        for level_echos in echos.clone().into_iter(){
            // Check if this level has only three interval values
            if level_echos.1.len() == 3{
                // Next, check if the interval range spans from MIN to MAX
                if level_echos.1.first().unwrap().0 == Val::MIN && level_echos.1.last().unwrap().1 == Val::MAX{
                    // Check if the mid level has value Max
                    let mid_level = level_echos.1.get(1).unwrap().clone();
                    if mid_level.2 == max_input{
                        // Mark the midpoint
                        if level_thresh == max_levels+1{
                            coalesce_val= (mid_level.0+mid_level.1)/2;
                            level_thresh = level_echos.0;
                        }
                    }
                }
            }
        }
        // Create a new level_echos message
        let mut level_echos_ret = Vec::new();
        for level_echos in echos.into_iter(){
            if level_echos.0<level_thresh{
                level_echos_ret.push(level_echos);
            }
        }
        ((coalesce_val,level_thresh,max_levels,round),level_echos_ret)
    }

    /**
     * This function contains the aggregation logic described in the Delphi protocol. 
     * It is invoked after total_rounds_bin rounds of Binary AA are terminated for all checkpoints at all levels. 
     * Check our paper for more details about the aggregation logic. 
     */
    pub fn aggregate(&self)->Val{
        let mut weights = Vec::new();
        let mut values = Vec::new();
        for level in 0..self.total_levels{
            let level_state = self.round_state.get(&level).unwrap();
            //let mut prev_lev_max_weight:i128 = 0;
            let mut max_lev_weight:i128 = 0;
            let mut weight_sum:i128 = 0;
            let mut weighted_sum:i128 = 0;
            for (_int_s,interval) in level_state.intervals.iter(){
                let term_weight = interval.term_value(self.total_rounds_bin) as i128;
                log::info!("Interval {}->{} in level {} has weight {}",interval.start,interval.end,level,term_weight);
                let midpoint = (interval.start+interval.end)/2;
                if term_weight> max_lev_weight{
                    max_lev_weight = term_weight as i128;
                }
                let term_weight = (interval.term_value(self.total_rounds_bin) as i128)*((interval.end-interval.start) as i128/level_state.sep as i128);
                weight_sum += term_weight as i128;
                weighted_sum += (midpoint as i128)*(term_weight as i128);
            }
            if weight_sum == 0{
                log::info!("Zero weight for level {}",level);
                weight_sum = self.epsilon as i128;
                weighted_sum = self.value as i128;
                max_lev_weight = self.epsilon as i128;
            }
            else{
                log::info!("Level weight:{}, level sum:{} for level {}",weight_sum,weighted_sum,level);
            }
            let weighted_avg = weighted_sum/weight_sum;
            values.push(weighted_avg);
            weights.push(max_lev_weight);
        }
        let mut level_wise_average:i128 = 0;
        let mut level_wise_weight:i128 = 0;
        log::info!("Final weights: {:?}, values:{:?}",weights,values);
        for level in 0..self.total_levels{
            let level_weight;
            if level>0{
                level_weight = weights.get(level as usize).unwrap()*((weights.get(level as usize).unwrap()-weights.get((level-1) as usize).unwrap()).abs());
                level_wise_average += values.get(level as usize).unwrap()*level_weight;
            }
            else {
                level_weight = *weights.get(level as usize).unwrap();
                level_wise_average += values.get(level as usize).unwrap()*level_weight;
            }
            level_wise_weight += level_weight;
        }
        let final_val = (level_wise_average/level_wise_weight) as Val;
        log::info!("Terminated Approximate Agreement protocol, output: {}",final_val);
        final_val
    }
}