use anyhow::{Result, anyhow};
use network::{plaintcp::{TcpReceiver, TcpReliableSender, CancelHandler}, Acknowledgement};
use tokio::sync::{oneshot, mpsc::{unbounded_channel, UnboundedReceiver}};
use types::{appxcon::{WrapperMsg, Replica, ProtMsg}, SyncMsg, SyncState, Val, Lev, Round};
use config::Node;
use fnv::FnvHashMap;
use std::{net::{SocketAddr, SocketAddrV4}, collections::HashMap, time::{SystemTime, UNIX_EPOCH}};

use super::{Handler, SyncHandler, Level, RBCState};

pub struct Context {
    /// Networking context
    pub net_send: TcpReliableSender<Replica,WrapperMsg,Acknowledgement>,
    pub net_recv: UnboundedReceiver<WrapperMsg>,
    pub sync_send:TcpReliableSender<Replica,SyncMsg,Acknowledgement>,
    pub sync_recv: UnboundedReceiver<SyncMsg>,
    
    /// Data context
    pub num_nodes: usize,
    pub myid: usize,
    pub num_faults: usize,
    pub payload:usize,

    /// PKI
    /// Replica map
    pub sec_key_map:HashMap<Replica, Vec<u8>>,

    /// Round number and Approx Consensus related context
    pub round:Round,
    pub value:Val,
    pub rho:Val,
    pub epsilon:Val,
    pub maxrange: Val,
    pub exponent: f32,

    pub min_rounds_bin:Round,
    pub total_rounds_bin:Round,
    pub total_levels: Lev,

    /// Optimistic termination
    pub start_rbc: bool,
    pub terminated:bool,
    pub rbc_term: Vec<Replica>,
    pub rbc_state: RBCState,

    pub input: Val,
    pub max_input:Val,
    /// State context
    pub round_state: HashMap<Lev,Level>,
    /// Exit protocol
    exit_rx: oneshot::Receiver<()>,
    /// Cancel Handlers
    pub cancel_handlers: HashMap<Round,Vec<CancelHandler<Acknowledgement>>>,
}

impl Context {
    pub fn spawn(
        config: Node,
        val: Val,
        epsilon: Val,
        rho:Val,
        maxrange: Val,
        exponent: f32
    ) -> anyhow::Result<oneshot::Sender<()>> {
        let prot_payload = &config.prot_payload;
        let v:Vec<&str> = prot_payload.split(',').collect();
        let mut consensus_addrs :FnvHashMap<Replica,SocketAddr>= FnvHashMap::default();
        for (replica,address) in config.net_map.iter(){
            let address:SocketAddr = address.parse().expect("Unable to parse address");
            consensus_addrs.insert(*replica, SocketAddr::from(address.clone()));
        }
        let mut syncer_map:FnvHashMap<Replica,SocketAddr> = FnvHashMap::default();
        syncer_map.insert(0, config.client_addr);
        let my_port = consensus_addrs.get(&config.id).unwrap();
        let my_address = to_socket_address("0.0.0.0", my_port.port());
        let syncer_listen_port = config.client_port;
        let syncer_l_address = to_socket_address("0.0.0.0", syncer_listen_port);
        
        // Setup networking
        let (tx_net_to_consensus, rx_net_to_consensus) = unbounded_channel();
        TcpReceiver::<Acknowledgement, WrapperMsg, _>::spawn(
            my_address,
            Handler::new(tx_net_to_consensus),
        );
        // The server must listen to the client's messages on some port that is not being used to listen to other servers
        let (tx_net_to_client,rx_net_from_client) = unbounded_channel();
        TcpReceiver::<Acknowledgement,SyncMsg,_>::spawn(
            syncer_l_address, 
            SyncHandler::new(tx_net_to_client)
        );
        // let _sleep_time = sleep - SystemTime::now().duration_since(UNIX_EPOCH)
        // .unwrap()
        // .as_millis();
        log::debug!("Consensus addrs {:?}",consensus_addrs);
        let consensus_net = TcpReliableSender::<Replica,WrapperMsg,Acknowledgement>::with_peers(
            consensus_addrs.clone()
        );
        let sync_net = TcpReliableSender::<Replica,SyncMsg,Acknowledgement>::with_peers(syncer_map);
        if v[0] == "a" {
            let (exit_tx, exit_rx) = oneshot::channel();
            tokio::spawn( async move {
                let prot_payload = &config.prot_payload;
                let v:Vec<&str> = prot_payload.split(',').collect();
                let _init_value:u64 = v[1].parse::<u64>().unwrap();
                // delta is the level of allowed overshoot, 
                // epsilon is the final state of disagreement

                let exponent:f32 = exponent;
                let levels = maxrange as f64/rho as f64;
                let exponent_log = (exponent as f64).log2();
                let levels = (levels.log2()/exponent_log).ceil() as Lev;
                let rounds = ((2*maxrange*(config.num_nodes as i64+3)*(levels as i64)) as f64/epsilon as f64).log2().ceil() as Round;
                //let rounds = (rounds/exponent_log).ceil() as Round;
                let min_rounds = ((2*rho*(config.num_nodes as i64+3)*(levels as i64)) as f64/epsilon as f64).log2().ceil() as Round;
                let max_input:Val = exponent.powf((rounds+1) as f32).ceil() as Val;
                log::info!("Min rounds:{}, Max rounds: {}",min_rounds,rounds);
                let mut levelmap:HashMap<Lev,Level> = HashMap::default();
                for level in 0..levels{
                    let sep = rho*((exponent.powf(level as f32).ceil()) as Val);
                    levelmap.insert(level, Level::new(sep, level, val, config.num_faults+1, config.num_nodes-config.num_faults));
                }
                // TODO: Estimate the number of rounds of approximate agreement needed
                let mut c = Context {
                    net_send: consensus_net,
                    net_recv: rx_net_to_consensus,
                    sync_send: sync_net,
                    sync_recv: rx_net_from_client,
                    num_nodes: config.num_nodes,
                    sec_key_map: HashMap::default(),
                    myid: config.id,
                    num_faults: config.num_faults,
                    payload: config.payload,
                    round:0,
                    value: val,
                    rho:rho,
                    epsilon: epsilon,
                    maxrange:maxrange,
                    exponent: exponent,

                    min_rounds_bin:min_rounds,
                    total_rounds_bin:rounds,
                    total_levels: levels,
                    input: val,
                    max_input:max_input,

                    start_rbc:false,
                    terminated:false,
                    rbc_term: Vec::new(),
                    rbc_state:RBCState::new(),

                    round_state: levelmap,
                    exit_rx:exit_rx,
                    cancel_handlers:HashMap::default()
                };
                for (id, sk_data) in config.sk_map.clone() {
                    c.sec_key_map.insert(id, sk_data.clone());
                }
                //c.invoke_coin.insert(100, Duration::from_millis(sleep_time.try_into().unwrap()));
                if let Err(e) = c.run().await {
                    log::error!("Consensus error: {}", e);
                }
                log::debug!("Started n-parallel RBC with value {} and epsilon {}",c.value,c.epsilon);
            });
            Ok(exit_tx)
        }
        else {
            panic!("Invalid configuration for protocol");
        }
    }

    pub async fn broadcast(&mut self, protmsg:ProtMsg){
        let sec_key_map = self.sec_key_map.clone();
        for (replica,sec_key) in sec_key_map.into_iter() {
            if replica != self.myid{
                let wrapper_msg = WrapperMsg::new(protmsg.clone(), self.myid, &sec_key.as_slice());
                let cancel_handler:CancelHandler<Acknowledgement> = self.net_send.send(replica, wrapper_msg).await;
                self.add_cancel_handler(cancel_handler);
            }
        }
    }

    pub async fn run(&mut self)-> Result<()>{
        // Send the client message that we are alive and kicking
        let cancel_handler = self.sync_send.send(
    0,
       SyncMsg { sender: self.myid, state: SyncState::ALIVE,value:0}).await;
        self.add_cancel_handler(cancel_handler);
        loop {
            tokio::select! {
                // Receive exit handlers
                exit_val = &mut self.exit_rx => {
                    exit_val.map_err(anyhow::Error::new)?;
                    log::info!("Termination signal received by the server. Exiting.");
                    break
                },
                msg = self.net_recv.recv() => {
                    // Received a protocol message
                    // Received a protocol message
                    log::debug!("Got a consensus message from the network: {:?}", msg);
                    let msg = msg.ok_or_else(||
                        anyhow!("Networking layer has closed")
                    )?;
                    self.process_msg(msg).await;
                },
                sync_msg = self.sync_recv.recv() =>{
                    let sync_msg = sync_msg.ok_or_else(||
                        anyhow!("Networking layer has closed")
                    )?;
                    match sync_msg.state {
                        SyncState::START =>{
                            log::error!("Consensus Start time: {:?}", SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_millis());
                            self.start_baa(0 as Round).await;
                            let cancel_handler = self.sync_send.send(0, SyncMsg { sender: self.myid, state: SyncState::STARTED, value:0}).await;
                            self.add_cancel_handler(cancel_handler);
                        },
                        SyncState::STOP =>{
                            log::error!("Consensus Stop time: {:?}", SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_millis());
                            log::info!("Termination signal received by the server. Exiting.");
                            break
                        },
                        _=>{}
                    }
                }
            }
        }
        Ok(())
    }
    pub fn add_cancel_handler(&mut self, canc: CancelHandler<Acknowledgement>){
        self.cancel_handlers
            .entry(self.round)
            .or_default()
            .push(canc);
    }
}

pub fn to_socket_address(
    ip_str: &str,
    port: u16,
) -> SocketAddr {
    let addr = SocketAddrV4::new(ip_str.parse().unwrap(), port);
    addr.into()
}