// Copyright (c) 2022 - for information on the respective copyright owner see the NOTICE file or the repository https://github.com/hyperledger/indy-node-container.
//
// SPDX-License-Identifier: Apache-2.0


use indy_credx::issuer::create_credential_definition;
use indy_credx::types::CredentialDefinitionConfig;
use indy_data_types::anoncreds::cred_def::{
    CredentialDefinition, CredentialDefinitionPrivate, SignatureType,
};
use indy_data_types::anoncreds::schema::Schema;
use indy_vdr::common::error::VdrResult;
use indy_vdr::ledger::RequestBuilder;
use indy_vdr::pool::PreparedRequest;
use indy_vdr::utils::did::DidValue;

pub fn generate_tx_cred_def(
    builder: &RequestBuilder,
    did: &DidValue,
    schema: &Schema,
    tag: &str,
) -> VdrResult<(
    PreparedRequest,
    CredentialDefinition,
    CredentialDefinitionPrivate,
)> {
    let (cred_def, private_key, _correctness_proof) = create_credential_definition(
        did,
        schema,
        tag,
        SignatureType::CL,
        CredentialDefinitionConfig::new(true),
    )
        .unwrap();

    let tx = builder.build_cred_def_request(did, cred_def);

    // TODO: Why does CredentialDefiniton not derive Clone?
    let (cred_def, _, _) = create_credential_definition(
        did,
        schema,
        tag,
        SignatureType::CL,
        CredentialDefinitionConfig::new(true),
    )
        .unwrap();
    return match tx {
        Ok(tx) => Ok((tx, cred_def, private_key)),
        Err(err) => Err(err),
    };
}