use crate::did::{DidValue, ShortDidValue};
use clap::Parser;
use futures_executor::block_on;
use indy_vdr::pool::helpers::perform_ledger_request;
use indy_vdr::pool::helpers::perform_refresh;
use indy_vdr::pool::{Pool, PoolBuilder, PoolTransactions, RequestResult};
use indy_vdr::utils::did;
use indy_vdr::utils::keys::VerKey;
use log::info;
use rand::{distributions::Alphanumeric, Rng};
use serde_json::Value;
use std::{thread, time};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Seed to sign transactions with
    #[clap(
        short = 's',
        long = "seed",
        default_value = "000000000000000000000000Trustee1"
    )]
    seed: String,

    /// Pool transaction genesis filename
    #[clap(
        short = 'g',
        long = "genesis",
        default_value = "/pool_transactions_genesis"
    )]
    genesis_file: String,
}

pub fn generate_seed() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .collect()
}

pub fn long_did(did: &ShortDidValue, verkey: &VerKey) -> DidValue {
    return did.qualify(Option::from(verkey.to_string()));
}

fn main() {
    let args = Args::parse();
    env_logger::init();

    let (trustee_did, trustee_pkey, trustee_ver_key) =
        did::generate_did(Option::from(args.seed.as_bytes())).unwrap();

    // Initialize pool
    info!("Initializing pool");
    let genesistxs = PoolTransactions::from_json_file(args.genesis_file.as_str()).unwrap();
    let pool_builder = PoolBuilder::default()
        .transactions(genesistxs.clone())
        .unwrap();
    let pool = pool_builder.into_shared().unwrap();

    // Refresh pool (if for some reason we add nodes later on)
    info!("Refreshing pool");
    let (txns, _timing) = block_on(perform_refresh(&pool)).unwrap();

    let pool = if let Some(txns) = txns {
        let builder = {
            let mut pool_txns = genesistxs;
            pool_txns.extend_from_json(&txns).unwrap();
            PoolBuilder::default()
                .transactions(pool_txns.clone())
                .unwrap()
        };
        builder.into_shared().unwrap()
    } else {
        pool
    };

    info!("Refreshed Pool: ");
    for node in pool.get_node_aliases() {
        info!("{}", node);
    }
    for element in pool.get_json_transactions().unwrap() {
        info!("{}", element);
    }

    let builder = pool.get_request_builder();
    loop {
        // Create random Seed
        let seed: String = generate_seed();
        let (did, _, verkey) = did::generate_did(Option::from(seed.as_bytes())).unwrap();
        // Create nym request from seed
        let mut req = builder
            .build_nym_request(
                &long_did(&trustee_did, &trustee_ver_key),
                &long_did(&did, &verkey),
                Option::from(verkey.to_string()),
                None,
                None,
                None,
                None,
            )
            .unwrap();
        req.set_signature(
            trustee_pkey
                .sign(req.get_signature_input().unwrap().as_bytes())
                .unwrap()
                .as_slice(),
        )
        .unwrap();

        let (result, _timing) = block_on(perform_ledger_request(&pool, &req)).unwrap();
        let result_data = match result {
            RequestResult::Reply(data) => Ok(data),
            RequestResult::Failed(error) => Err(error),
        };
        let v: Value = serde_json::from_str(result_data.unwrap().as_str()).unwrap();
        info!("Got Raw Value: {}", v);
        let data: &Value = &v["result"]["data"].to_owned();
        info!("Got Value: {}", data);
        thread::sleep(time::Duration::from_millis(100));
    }
}
