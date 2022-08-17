// Copyright (c) 2022 - for information on the respective copyright owner see the NOTICE file or the repository https://github.com/hyperledger/indy-node-container.
//
// SPDX-License-Identifier: Apache-2.0

use indy_vdr::common::error::VdrResult;
use indy_vdr::ledger::RequestBuilder;
use indy_vdr::pool::PreparedRequest;
use indy_vdr::utils::did;
use indy_vdr::utils::did::{DidValue, ShortDidValue};
use indy_vdr::utils::keys::{PrivateKey, VerKey};
use rand::{distributions::Alphanumeric, Rng};

// Create a DidValue ( long form)
pub fn long_did(did: &ShortDidValue) -> DidValue {
    return did.qualify(Option::from("sov".to_owned()));
}

// Generate random seed used to create public/private key
pub fn generate_seed() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .map(char::from)
        .take(32)
        .collect()
}

// Create unsigned NYM Transaction
pub fn generate_tx_nym(
    builder: &RequestBuilder,
    trustee_did: &DidValue,
) -> VdrResult<(PreparedRequest, DidValue, PrivateKey, VerKey)> {
    // Create random Seed
    let seed: String = generate_seed();
    let (did, private_key, ver_key) = did::generate_did(Option::from(seed.as_bytes()))?;
    let qualified_did = long_did(&did);
    // Create nym request from seed
    let tx = builder.build_nym_request(
        trustee_did,
        &qualified_did,
        Some(ver_key.to_string()),
        None,
        Some("101".to_owned()),
    )?;

    Ok((tx, qualified_did, private_key, ver_key))
}
