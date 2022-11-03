// Copyright (c) 2022 - for information on the respective copyright owner see the NOTICE file or the repository https://github.com/hyperledger/indy-node-container.
//
// SPDX-License-Identifier: Apache-2.0

use indy_credx::issuer::create_schema;
use indy_data_types::anoncreds::schema::{AttributeNames, Schema};
use indy_vdr::common::error::VdrResult;
use indy_vdr::ledger::RequestBuilder;
use indy_vdr::pool::PreparedRequest;
use indy_vdr::utils::did::DidValue;
use std::collections::HashSet;

// Static attributes for every schema
static ATTRIBUTES: &'static [&str] = &["name", "email", "attr1", "attr2", "attr3", "attr4"];

pub fn generate_tx_schema(
    builder: &RequestBuilder,
    did: &DidValue,
) -> VdrResult<(PreparedRequest, Schema)> {
    let version = "0.1.0";
    let name = "TestSchema";
    let mut attributes = HashSet::new();
    for attribute in ATTRIBUTES {
        attributes.insert(attribute.to_string());
    }

    let schema = create_schema(did, name, version, AttributeNames(attributes), None).unwrap();
    let tx = builder.build_schema_request(did, schema.to_owned());

    return match tx {
        Ok(tx) => Ok((tx, schema)),
        Err(err) => Err(err),
    };
}
