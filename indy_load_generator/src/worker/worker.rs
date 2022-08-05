// Copyright (c) 2022 - for information on the respective copyright owner see the NOTICE file or the repository https://github.com/hyperledger/indy-node-container.
//
// SPDX-License-Identifier: Apache-2.0

use crate::thread::CloseReceiver;
use crate::worker::{nym, schema};
use futures::{pin_mut, select, FutureExt, StreamExt};
use futures_executor::block_on;
use indy_data_types::anoncreds::schema::SchemaV1;
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
use std::thread;
use tokio::runtime::{Builder, Runtime};

/// Store Information about writen information
#[derive(Debug, Clone)]
struct WriteInformation {
    did: Option<DidValue>,
    schema: Option<SchemaV1>,
}

impl WriteInformation {
    fn new() -> WriteInformation {
        return WriteInformation {
            did: None,
            schema: None,
        };
    }
}

/// utility class for a worker creating and sending transactions to a indy ledger
pub struct IndyWorker {
    pool: SharedPool,
    trustee_pkey: PrivateKey,
    trustee_qualified: DidValue,
    req_builder: RequestBuilder,
    id: String,
    read_ratio: i8,

    writes: u64,
    reads: u64,

    runtime: Runtime,
}

impl IndyWorker {
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
        read_ratio: i8,
    ) -> Result<IndyWorker, Box<dyn Error>> {
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

        let runtime = Builder::new_current_thread().enable_all().build().unwrap();

        Ok(IndyWorker {
            req_builder,
            trustee_qualified,
            pool,
            trustee_pkey,
            id,
            read_ratio,
            writes: 0,
            reads: 0,
            runtime,
        })
    }

    fn write(&mut self) -> WriteInformation {
        let mut write = WriteInformation::new();

        // Nym Transaction
        let tx_nym = nym::generate_tx_nym(&self.req_builder, &self.trustee_qualified);
        let (req, did, did_private_key, _ver_key) = match tx_nym {
            Ok(tx) => tx,
            Err(err) => {
                error!("[{}] Could not generate nym transaction: {}", self.id, err);
                return write;
            }
        };

        let nym_result = self.sign_and_send(req, self.trustee_pkey.to_owned());
        if nym_result.is_err() {
            error!(
                "[{}] Could not sign or send nym transaction: {}",
                self.id,
                nym_result.err().unwrap()
            );
            return write;
        }
        write.did = Some(did.to_owned());
        self.writes = self.writes + 1;

        // Schema Transaction
        let tx_schema = schema::generate_tx_schema(&self.req_builder, &did.to_owned());
        let (req, schema) = match tx_schema {
            Ok(tx) => tx,
            Err(err) => {
                error!(
                    "[{}] Could not generate schema transaction: {}",
                    self.id, err
                );
                return write;
            }
        };
        let schema_result = self.sign_and_send(req, did_private_key.to_owned());
        if schema_result.is_err() {
            error!(
                "[{}] Could not sign or send nym transaction: {}",
                self.id,
                schema_result.err().unwrap()
            );
            return write;
        }
        write.schema = Some(schema);
        self.writes = self.writes + 1;
        return write;
    }

    fn read_loop(&mut self, info: &WriteInformation) {
        for x in 1..self.read_ratio {}
    }

    async fn write_and_read(&mut self) {
        let info = self.write();
        self.read_loop(&info);
    }

    // Main function for the worker
    pub async fn run_loop(&mut self, receiver: &mut CloseReceiver) -> (u64, u64) {
        let id = self.id.to_owned();
        info!("[{}] Starting main loop", id);
        loop {
            let (w, r) = (self.writes, self.reads);
            let task = self.write_and_read().fuse();
            pin_mut!(task);
            select! {
                // Listen to incoming commands
                _ = receiver.next() => {
                    info!("[{}] Terminating main loop", id);
                    {
                    return (w, r);
                    }
                }
                // Start writing/reading
                _ = task => {

                }
            };
            thread::yield_now();
        }
    }

    // Helper function to sign and send transactions
    fn sign_and_send(&self, mut req: PreparedRequest, private_key: PrivateKey) -> VdrResult<()> {
        // Create Signature
        req.set_signature(
            private_key
                .sign(req.get_signature_input()?.as_bytes())?
                .as_slice(),
        )?;
        // Send transaction to ledger
        let (res, _) = self
            .runtime
            .block_on(perform_ledger_request(&self.pool, &req))?;
        match res {
            RequestResult::Reply(data) => {
                debug!("Sent data to ledger: {}", data);
                Ok(())
            }
            RequestResult::Failed(error) => Err(error),
        }
    }
}
