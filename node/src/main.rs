use anyhow::{Result,anyhow};
use clap::{
    load_yaml, 
    App
};
use config::Node;
use fnv::FnvHashMap;
use node::Syncer;
use signal_hook::{iterator::Signals, consts::{SIGINT, SIGTERM}};
use types::Val;
use std::{net::{SocketAddr, SocketAddrV4}};

#[tokio::main]
async fn main() -> Result<()> {
    log::error!("{}", std::env::current_dir().unwrap().display());
    let yaml = load_yaml!("cli.yml");
    let m = App::from_yaml(yaml).get_matches();
    //println!("{:?}",m);
    let conf_str = m.value_of("config")
        .expect("unable to convert config file into a string");
    let vss_type = m.value_of("vsstype")
        .expect("Unable to detect VSS type");
    let sleep = m.value_of("sleep")
        .expect("Unable to detect sleep time").parse::<u128>().unwrap();
    let _batch = m.value_of("batch")
        .expect("Unable to parse batch size").parse::<usize>().unwrap();
    let val_appx = m.value_of("val")
        .expect("Value required").parse::<Val>().unwrap();
    let delta = m.value_of("delta")
        .expect("Value required").parse::<Val>().unwrap();
    let epsilon = m.value_of("epsilon")
        .expect("Value required").parse::<Val>().unwrap();
    let tri = m.value_of("tri")
        .expect("Value required").parse::<Val>().unwrap();
    let syncer_file = m.value_of("syncer")
        .expect("Unable to parse syncer ip file");
    let rand = m.value_of("rand")
        .expect("Unable to parse random number").parse::<usize>().unwrap();
    let expo = m.value_of("expo")
        .expect("Unable to parse exponent").parse::<f32>().unwrap();
    let conf_file = std::path::Path::new(conf_str);
    let str = String::from(conf_str);
    let mut config = match conf_file
        .extension()
        .expect("Unable to get file extension")
        .to_str()
        .expect("Failed to convert the extension into ascii string") 
    {
        "json" => Node::from_json(str),
        "dat" => Node::from_bin(str),
        "toml" => Node::from_toml(str),
        "yaml" => Node::from_yaml(str),
        _ => panic!("Invalid config file extension"),
    };

    simple_logger::SimpleLogger::new().with_utc_timestamps().init().unwrap();
    // match m.occurrences_of("debug") {
    //     0 => log::set_max_level(log::LevelFilter::Info),
    //     1 => log::set_max_level(log::LevelFilter::Debug),
    //     2 | _ => log::set_max_level(log::LevelFilter::Trace),
    // }
    log::info!("epsilon: {:?},delta: {:?},value: {:?}, tri:{:?}",epsilon,delta,val_appx,tri);
    log::set_max_level(log::LevelFilter::Info);
    config
        .validate()
        .expect("The decoded config is not valid");
    if let Some(f) = m.value_of("ip") {
        let f_str = f.to_string();
        log::info!("Logging the file f {}",f_str);
        config.update_config(util::io::file_to_ips(f.to_string()));
    }
    let config = config;
    // Start the Reliable Broadcast protocol
    let exit_tx;
    match vss_type{
        // "ped" =>{
        //     //exit_tx = pedavss_cc::node::Context::spawn(config,sleep).unwrap();
        // },
        // "fre" => {
        //     //exit_tx = hash_cc::node::Context::spawn(config,sleep).unwrap();
        // },
        // "hr" => {
        //     //exit_tx = hash_cc_baa::node::Context::spawn(config,sleep,batch).unwrap();
        // },
        "appx" => {
            exit_tx = appxcon::node::Context::spawn(config, sleep, val_appx as u64,epsilon as u64).unwrap();
        },
        "hyb" =>{
            exit_tx = hyb_appxcon::node::Context::spawn(config,sleep,val_appx as u64,delta as u64,epsilon as u64,tri as u64).unwrap();
        },
        "del" =>{
            exit_tx = delphi::node::Context::spawn(config,val_appx,epsilon,delta,tri,expo).unwrap();
        },
        "delrbc" =>{
            exit_tx = delphi_rbc::node::Context::spawn(config,val_appx,epsilon,delta,tri,expo).unwrap();
        },
        "fin" =>{
            let rand = rand.to_string();
            let mut arr_strsplit:Vec<&str> = conf_str.split("/").collect();
            let id_str = ((config.id +1)).to_string();
            //let id_str_1  = ((config.id)).to_string();
            let key_str = "sec".to_string();
            
            let concat_str = key_str + &id_str;
            let _last_elem = arr_strsplit.pop();

            let mut vec_native = Vec::new();
            for i in 1..config.num_nodes+1{
                let pkey_str = "pub".to_string();
                let mut tpub = arr_strsplit.clone();
                let iter_str = pkey_str.clone()+ &(i.to_string());
                tpub.push(iter_str.as_str());
                vec_native.push(tpub.join("/"));
            }
            arr_strsplit.push(concat_str.as_str());
            println!("{:?} {:?}", arr_strsplit.join("/").as_str(), vec_native);
            exit_tx = fin::node::Context::spawn(
                config, 
                arr_strsplit.join("/").as_str(),
                vec_native,
                val_appx,
                rand
            ).unwrap();
        },
        "sync" => {
            let f_str = syncer_file.to_string();
            log::info!("Logging the file f {}",f_str);
            let ip_str = util::io::file_to_ips(f_str);
            let mut net_map = FnvHashMap::default();
            let mut idx = 0;
            for ip in ip_str{
                net_map.insert(idx, ip.clone());
                idx += 1;
            }
            //let client_addr = net_map.get(&(net_map.len()-1)).unwrap();
            exit_tx = Syncer::spawn(net_map, config.client_addr.clone()).unwrap();
        },
        _ =>{
            log::error!("Matching VSS not provided {}, canceling execution",vss_type);
            return Ok(());
        }
    }
    //let exit_tx = pedavss_cc::node::Context::spawn(config).unwrap();
    // Implement a waiting strategy
    let mut signals = Signals::new(&[SIGINT, SIGTERM])?;
    signals.forever().next();
    log::error!("Received termination signal");
    exit_tx
        .send(())
        .map_err(|_| anyhow!("Server already shut down"))?;
    log::error!("Shutting down server");
    Ok(())
}



pub fn to_socket_address(
    ip_str: &str,
    port: u16,
) -> SocketAddr {
    let addr = SocketAddrV4::new(ip_str.parse().unwrap(), port);
    addr.into()
}