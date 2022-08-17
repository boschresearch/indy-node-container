// Copyright (c) 2022 - for information on the respective copyright owner see the NOTICE file or the repository https://github.com/hyperledger/indy-node-container.
//
// SPDX-License-Identifier: Apache-2.0

use indy_data_types::anoncreds::schema::{AttributeNames, Schema, SchemaV1};
use indy_data_types::SchemaId;
use indy_vdr::common::error::VdrResult;
use indy_vdr::ledger::RequestBuilder;
use indy_vdr::pool::PreparedRequest;
use indy_vdr::utils::did::DidValue;
use std::collections::HashSet;

// Static attributes for every schema
static ATTRIBUTES: &'static [&str] = &["name", "email", "attr1", "attr2", "attr3", "attr4"];

// Create unsigned NYM Transaction
pub fn generate_tx_schema(
    builder: &RequestBuilder,
    did: &DidValue,
) -> VdrResult<(PreparedRequest, SchemaV1)> {
    let version = "0.1.0";
    let name = "TestSchema";
    let mut attributes = HashSet::new();
    for attribute in ATTRIBUTES {
        attributes.insert(attribute.to_string());
    }

    let schema = SchemaV1 {
        id: SchemaId::new(did, name, version),
        name: name.to_string(),
        version: version.to_string(),
        attr_names: AttributeNames(attributes),
        seq_no: None,
    };
    let tx = builder.build_schema_request(did, Schema::SchemaV1(schema.to_owned()));

    return match tx {
        Ok(tx) => Ok((tx, schema)),
        Err(err) => Err(err),
    };
}
