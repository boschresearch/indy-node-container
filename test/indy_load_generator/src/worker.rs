
use std::time::Duration;
use futures_executor::block_on;
use indy_vdr::common::error::VdrResult;
use indy_vdr::ledger::RequestBuilder;
use indy_vdr::pool::helpers::perform_ledger_request;
use indy_vdr::pool::helpers::perform_refresh;
use indy_vdr::pool::{Pool, PoolBuilder, PoolTransactions, PreparedRequest, RequestResult};
use indy_vdr::utils::did;
use indy_vdr::utils::did::{DidValue, ShortDidValue};
use indy_vdr::utils::keys::VerKey;
use log::{info, debug, error};
use rand::{distributions::Alphanumeric, Rng};


fn long_did(did: &ShortDidValue) -> DidValue {
    return did.qualify(Option::from("sov".to_owned()));
}

fn generate_seed() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .collect()
}

fn generate_tx_nym(builder: &RequestBuilder, trustee_did: &DidValue) -> VdrResult<(PreparedRequest, DidValue, VerKey)> {
    // Create random Seed
    let seed: String = generate_seed();
    let (did, _, verkey) = did::generate_did(Option::from(seed.as_bytes()))?;
    let qualified_did = long_did(&did);
    // Create nym request from seed
    let tx = builder.build_nym_request(
           trustee_did,
          &qualified_did,
          Option::from(verkey.to_string()),
          None,
          Option::from("101".to_owned()),
          None,
          None,
      )?;

    Ok((tx, qualified_did, verkey))
}


pub fn run(seed: String, genesis_path: String, duration: Duration, id: i32) -> VdrResult<()> {
    info!("Starting worker [{}]", id);
    let (trustee_did, trustee_pkey, _) =
        did::generate_did(Option::from(seed.as_bytes()))?;
    let trustee_qualified = long_did(&trustee_did);

    // Initialize pool
    let genesistxs = PoolTransactions::from_json_file(genesis_path)?;
    let pool_builder = PoolBuilder::default()
        .transactions(genesistxs.clone())?;
    let pool = pool_builder.into_shared()?;

    // Refresh pool (if for some reason we add nodes later on)
    let (txns, _timing) = block_on(perform_refresh(&pool))?;

    let pool = if let Some(txns) = txns {
        let builder = {
            let mut pool_txns = genesistxs;
            pool_txns.extend_from_json(&txns)?;
            PoolBuilder::default()
                .transactions(pool_txns.clone())?
        };
        builder.into_shared()?
    } else {
        pool
    };

    let req_builder = &pool.get_request_builder();

    // Pool updated & builder ready -> start sending
    loop {
        let (mut req, did, verkey) = generate_tx_nym(req_builder, &trustee_qualified)?;
        // Create Signature
        req.set_signature(
            trustee_pkey
                .sign(req.get_signature_input().unwrap().as_bytes())
                .unwrap()
                .as_slice(),
        )?;
        // Send transaction to ledger
        let (res, _) = block_on(perform_ledger_request(&pool, &req))?;
        match res {
            RequestResult::Reply(data) => {
                info!("[{}] Wrote nym {}: {}", id, did, data)
            }
            RequestResult::Failed(error) => {
                error!("[{}] Could not write nym {}: {}", id, did, error)
            }
        }
    }
}