// Copyright (c) 2022 - for information on the respective copyright owner see the NOTICE file or the repository https://github.com/hyperledger/indy-node-container.
//
// SPDX-License-Identifier: Apache-2.0

use indy_credx::issuer::{create_revocation_registry, update_revocation_registry};
use indy_credx::tails::{TailsFileReader, TailsFileWriter};
use indy_data_types::anoncreds::cred_def::CredentialDefinition;
use indy_data_types::anoncreds::rev_reg::{RevocationRegistry, RevocationRegistryDelta};
use indy_data_types::anoncreds::rev_reg_def::{
    IssuanceType, RegistryType, RevocationRegistryDefinition, RevocationRegistryDefinitionPrivate,
};
use indy_data_types::RevocationRegistryId;
use indy_vdr::common::error::VdrResult;
use indy_vdr::ledger::RequestBuilder;
use indy_vdr::pool::PreparedRequest;
use indy_vdr::utils::did::DidValue;
use std::collections::BTreeSet;

pub fn generate_tx_rev_reg_def(
    builder: &RequestBuilder,
    did: &DidValue,
    cred_def: &CredentialDefinition,
    tag: &str,
    size: u32,
) -> VdrResult<(
    PreparedRequest,
    RevocationRegistryDefinition,
    RevocationRegistryDefinitionPrivate,
    RevocationRegistry,
    RevocationRegistryDelta,
)> {
    // default to temp folder
    let mut writer = TailsFileWriter::new(None);
    let (rev_reg_def, rev_reg_def_private, rev_reg, reg_reg_delta) = create_revocation_registry(
        did,
        cred_def,
        tag,
        RegistryType::CL_ACCUM,
        IssuanceType::ISSUANCE_BY_DEFAULT,
        size,
        &mut writer,
    )
    .unwrap();
    let tx = builder.build_revoc_reg_def_request(did, rev_reg_def.clone());

    return match tx {
        Ok(tx) => Ok((tx, rev_reg_def, rev_reg_def_private, rev_reg, reg_reg_delta)),
        Err(err) => Err(err),
    };
}

pub fn generate_tx_init_rev_reg(
    builder: &RequestBuilder,
    did: &DidValue,
    rev_reg_def: &RevocationRegistryDefinition,
    rev_reg: &RevocationRegistry,
) -> VdrResult<PreparedRequest> {
    let tx = builder.build_revoc_reg_entry_request(
        did,
        rev_reg_def.id(),
        &RegistryType::CL_ACCUM,
        rev_reg.initial_delta(),
    );
    return match tx {
        Ok(tx) => Ok(tx),
        Err(err) => Err(err),
    };
}

pub fn generate_tx_update_rev_reg_entry(
    builder: &RequestBuilder,
    did: &DidValue,
    rev_reg: &RevocationRegistry,
    rev_reg_def: &RevocationRegistryDefinition,
    revoked: impl Iterator<Item = i64>,
) -> VdrResult<(PreparedRequest, RevocationRegistry)> {
    let path = match rev_reg_def {
        RevocationRegistryDefinition::RevocationRegistryDefinitionV1(def) => {
            def.value.tails_location.as_str()
        }
    };
    let revoked_set = revoked.into_iter().fold(BTreeSet::new(), |mut tree, i| {
        tree.insert(i as u32);
        tree
    });
    let reader = TailsFileReader::new(path);
    let (rev_reg, rev_reg_delta) =
        update_revocation_registry(rev_reg_def, rev_reg, BTreeSet::new(), revoked_set, &reader)
            .unwrap();
    let tx = builder.build_revoc_reg_entry_request(
        did,
        rev_reg_def.id(),
        &RegistryType::CL_ACCUM,
        rev_reg_delta,
    );
    return match tx {
        Ok(tx) => Ok((tx, rev_reg)),
        Err(err) => Err(err),
    };
}

pub fn generate_tx_get_delta(
    builder: &RequestBuilder,
    rev_reg_def_id: &RevocationRegistryId,
) -> VdrResult<PreparedRequest> {
    let tx = builder.build_get_revoc_reg_delta_request(
        None,
        rev_reg_def_id,
        None,
        chrono::offset::Utc::now().timestamp(),
    );
    return match tx {
        Ok(tx) => Ok(tx),
        Err(err) => Err(err),
    };
}
