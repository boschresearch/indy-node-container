// Copyright (c) 2022 - for information on the respective copyright owner see the NOTICE file or the repository https://github.com/hyperledger/indy-node-container.
//
// SPDX-License-Identifier: Apache-2.0

use crate::thread::CloseReceiver;
use crate::worker::{cred_def, nym, rev_reg, schema};
use futures::{pin_mut, select, FutureExt, StreamExt};
use futures_executor::block_on;
use indy_data_types::{CredentialDefinitionId, RevocationRegistryId, SchemaId};
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
use indy_data_types::anoncreds::schema::Schema::SchemaV1;

/// Store Information about writen information
#[derive(Debug, Clone)]
struct WriteInformation {
    did: Option<DidValue>,
    schema_id: Option<SchemaId>,
    cred_def_id: Option<CredentialDefinitionId>,
    rev_reg_def_id: Option<RevocationRegistryId>,
    rev_entries: u32,
}

impl WriteInformation {
    fn new() -> WriteInformation {
        return WriteInformation {
            did: None,
            schema_id: None,
            cred_def_id: None,
            rev_reg_def_id: None,
            rev_entries: 0,
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
    read_ratio: u32,
    revocation_entries: u32,

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
        read_ratio: u32,
        revocation_entries: u32,
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
            revocation_entries,
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

        let nym_result = self.sign_and_send(req, Some(&self.trustee_pkey));
        match nym_result {
            Ok(data) => {
                debug!("[{}] sent nym transaction", self.id)
            }
            Err(err) => {
                error!(
                    "[{}] Could not sign or send nym transaction: {}",
                    self.id, err
                );
                return write;
            }
        }
        write.did = Some(did.to_owned());
        self.writes = self.writes + 1;

        // Schema Transaction
        let tx_schema = schema::generate_tx_schema(&self.req_builder, &did);
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
        let schema_result = self.sign_and_send(req, Some(&did_private_key));
        let resp = match schema_result {
            Ok(data) => {
                debug!("[{}] sent schema transaction", self.id);
                data
            }
            Err(err) => {
                error!(
                    "[{}] Could not sign or send schema transaction: {}",
                    self.id, err
                );
                return write;
            }
        };
        write.schema_id = Some(schema.id().to_owned());
        self.writes = self.writes + 1;

        // Add seq_no to schema
        let res: serde_json::Value = serde_json::from_str(&resp).unwrap();
        let seq_no = res["result"]["txnMetadata"]["seqNo"].as_u64().unwrap();

        let schema = match schema {
            SchemaV1(s) => {
                let mut schema = s.clone();
                schema.seq_no = Some(seq_no as u32);
                SchemaV1(schema)
            }
        };

        // CredDef Transaction
        let tx_cred_def =
            cred_def::generate_tx_cred_def(&self.req_builder, &did, &schema, "testcred");
        let (req, cred_def, _) = match tx_cred_def {
            Ok(tx) => tx,
            Err(err) => {
                error!(
                    "[{}] Could not generate cred_def transaction: {}",
                    self.id, err
                );
                return write;
            }
        };
        let cred_def_result = self.sign_and_send(req, Some(&did_private_key));
        match cred_def_result {
            Ok(data) => {
                debug!("[{}] sent cred_def transaction", self.id);
            }
            Err(err) => {
                error!(
                    "[{}] Could not sign or send cred_def transaction: {}",
                    self.id, err
                );
                return write;
            }
        }
        write.cred_def_id = Some(cred_def.id().to_owned());
        self.writes = self.writes + 1;

        // Revocation Registry Definition
        let tx_rev_reg_def = rev_reg::generate_tx_rev_reg_def(
            &self.req_builder,
            &did.to_owned(),
            &cred_def,
            "1.0",
            self.revocation_entries + 5,
        );
        let (req, rev_reg_def, _rev_reg_def_priv, mut rev_reg, _rev_reg_delta) = match tx_rev_reg_def
        {
            Ok(tx) => tx,
            Err(err) => {
                error!(
                    "[{}] Could not generate rev_reg_def transaction: {}",
                    self.id, err
                );
                return write;
            }
        };
        let rev_reg_def_result = self.sign_and_send(req, Some(&did_private_key));
        match rev_reg_def_result {
            Ok(data) => {
                debug!("[{}] sent rev_reg_def transaction", self.id);
            }
            Err(err) => {
                error!(
                    "[{}] Could not sign or send rev_reg_def transaction: {}",
                    self.id, err
                );
                return write;
            }
        }
        write.rev_reg_def_id = Some(rev_reg_def.id().to_owned());
        self.writes = self.writes + 1;

        // Revocation Registry Init
        let tx_rev_reg = rev_reg::generate_tx_init_rev_reg(
            &self.req_builder,
            &did.to_owned(),
            &rev_reg_def,
            &rev_reg,
        );
        let req = match tx_rev_reg {
            Ok(tx) => tx,
            Err(err) => {
                error!(
                    "[{}] Could not generate rev_reg transaction: {}",
                    self.id, err
                );
                return write;
            }
        };
        let rev_reg_result = self.sign_and_send(req, Some(&did_private_key));
        match rev_reg_result {
            Ok(data) => {
                debug!("[{}] sent rev_reg transaction", self.id)
            }
            Err(err) => {
                error!(
                    "[{}] Could not sign or send rev_reg transaction: {}",
                    self.id, err
                );
                return write;
            }
        }
        self.writes = self.writes + 1;

        for x in 1..self.revocation_entries {
            // Revocation Registry Entries
            let tx_rev_reg_entry = rev_reg::generate_tx_update_rev_reg_entry(
                &self.req_builder,
                &did.to_owned(),
                &rev_reg,
                &rev_reg_def,
                vec![x as i64].into_iter(),
            );
            let (req, rev_reg_updated) = match tx_rev_reg_entry {
                Ok(tx) => tx,
                Err(err) => {
                    error!(
                        "[{}] Could not generate rev_reg_entry transaction: {}",
                        self.id, err
                    );
                    return write;
                }
            };
            let rev_reg_result = self.sign_and_send(req, Some(&did_private_key));
            match rev_reg_result {
                Ok(data) => {
                    debug!("[{}] sent rev_reg_entry transaction", self.id)
                }
                Err(err) => {
                    error!(
                        "[{}] Could not sign or send rev_reg_entry transaction: {}",
                        self.id, err
                    );
                    return write;
                }
            }
            rev_reg = rev_reg_updated;
            write.rev_entries = write.rev_entries + 1;
            self.writes = self.writes + 1;
        }
        return write;
    }

    fn read(&mut self, info: &WriteInformation) {
        for x in 1..self.read_ratio {

        }
    }

    async fn write_and_read(&mut self) {
        let info = self.write();
        self.read(&info);
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
    fn sign_and_send(&self, mut req: PreparedRequest, private_key: Option<&PrivateKey>) -> VdrResult<String> {
        // Create Signature
        match private_key {
            Some(private_key) => {
                req.set_signature(
                    private_key
                        .sign(req.get_signature_input()?.as_bytes())?
                        .as_slice(),
                )?;
            },
            None => {},
        }

        // Send transaction to ledger
        let (res, _) = self
            .runtime
            .block_on(perform_ledger_request(&self.pool, &req))?;
        match res {
            RequestResult::Reply(data) => {
                debug!("Sent data to ledger: {}", data);
                Ok(data)
            }
            RequestResult::Failed(error) => Err(error),
        }
    }
}
