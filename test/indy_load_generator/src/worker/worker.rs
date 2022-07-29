// Copyright (c) 2022 - for information on the respective copyright owner see the NOTICE file or the repository https://github.com/hyperledger/indy-node-container.
//
// SPDX-License-Identifier: Apache-2.0

use crate::worker::{nym, schema};
use futures_executor::block_on;
use indy_vdr::common::error::VdrResult;
use indy_vdr::ledger::RequestBuilder;
use indy_vdr::pool::{
    helpers::perform_ledger_request, helpers::perform_refresh, Pool, PoolBuilder, PoolTransactions,
    PreparedRequest, RequestResult, SharedPool,
};
use indy_vdr::utils::did;
use indy_vdr::utils::did::DidValue;
use indy_vdr::utils::keys::PrivateKey;
use log::{debug, error, info};
use std::error::Error;

/// utility class for a worker creating and sending transactions to a indy ledger
pub struct Worker {
    pool: SharedPool,
    trustee_pkey: PrivateKey,
    trustee_qualified: DidValue,
    req_builder: RequestBuilder,
    id: String,
}

impl Worker {
    /// Returns a worker that creates transactions for a provided indy network
    ///
    /// # Arguments
    ///
    /// * `seed` - A string that provides the seed of a registered DID that is allowed to register
    /// DIDs on the indy network
    ///
    /// * `genesis_path` - A string that provides the path to a pool transactions genesis file
    /// for the indy ledger under test
    ///
    /// * `id` - A string that is used for logging output to identity the worker
    pub fn new(
        seed: String,
        genesis_path: String,
        id: String,
    ) -> Result<Box<Worker>, Box<dyn Error>> {
        let (trustee_did, trustee_pkey, _) = did::generate_did(Option::from(seed.as_bytes()))?;
        let trustee_qualified = nym::long_did(&trustee_did);

        // Initialize pool
        let genesis_txs = PoolTransactions::from_json_file(genesis_path)?;
        let pool_builder = PoolBuilder::default().transactions(genesis_txs.clone())?;
        let pool = pool_builder.into_shared()?;

        // Refresh pool (to get current state of the pool ledger)
        let (genesis_txns, _timing) = block_on(perform_refresh(&pool))?;

        let pool = if let Some(txns) = genesis_txns {
            let builder = {
                let mut pool_txns = genesis_txs;
                pool_txns.extend_from_json(&txns)?;
                PoolBuilder::default().transactions(pool_txns.clone())?
            };
            builder.into_shared()?
        } else {
            pool
        };

        let req_builder = pool.get_request_builder();
        Ok(Box::new(Worker {
            req_builder,
            trustee_qualified,
            pool,
            trustee_pkey,
            id,
        }))
    }

    // Main function for the worker
    pub fn run(&self) {
        debug!("[{}] Starting worker", self.id);

        loop {
            // Nym Transaction
            let tx_nym = nym::generate_tx_nym(&self.req_builder, &self.trustee_qualified);
            let (req, did, did_private_key, _ver_key) = match tx_nym {
                Ok(tx) => tx,
                Err(err) => {
                    error!("[{}] Could not generate nym transaction: {}", self.id, err);
                    continue;
                }
            };
            let nym_result = self.sign_and_send(req, self.trustee_pkey.to_owned());
            if nym_result.is_err() {
                error!(
                    "[{}] Could not sign or send nym transaction: {}",
                    self.id,
                    nym_result.err().unwrap()
                );
                continue;
            }
            // Schema Transaction
            let tx_schema = schema::generate_tx_schema(&self.req_builder, &did);
            let (req, _schema) = match tx_schema {
                Ok(tx) => tx,
                Err(err) => {
                    error!(
                        "[{}] Could not generate schema transaction: {}",
                        self.id, err
                    );
                    continue;
                }
            };
            let schema_result = self.sign_and_send(req, did_private_key.to_owned());
            if schema_result.is_err() {
                error!(
                    "[{}] Could not sign or send nym transaction: {}",
                    self.id,
                    schema_result.err().unwrap()
                );
                continue;
            }

            let _ = tokio::task::yield_now();
        }
    }

    // Helper function to sign and send transactions
    fn sign_and_send(&self, mut req: PreparedRequest, private_key: PrivateKey) -> VdrResult<()> {
        // Create Signature
        req.set_signature(
            private_key
                .sign(req.get_signature_input().unwrap().as_bytes())
                .unwrap()
                .as_slice(),
        )?;
        // Send transaction to ledger
        let (res, _) = block_on(perform_ledger_request(&self.pool, &req))?;
        match res {
            RequestResult::Reply(data) => {
                debug!("Sent data to ledger: {}", data);
                Ok(())
            }
            RequestResult::Failed(error) => Err(error),
        }
    }
}

// Creates a worker and starts running
pub async fn spawn_worker(seed: String, genesis_path: String, name: String) {
    let worker_result = Worker::new(seed, genesis_path, name.to_owned());
    match worker_result {
        Ok(worker) => {
            info!("[{}] Worker created, start running", name);
            worker.run();
        }
        Err(e) => {
            error!("[{}] Could not initialize worker: {}", name, e)
        }
    }
}
