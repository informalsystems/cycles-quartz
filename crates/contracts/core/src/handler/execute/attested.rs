use ciborium::{from_reader as from_cbor_slice, into_writer as into_cbor, Value as CborValue};
use cosmwasm_std::{
    to_json_binary, Deps, DepsMut, Env, MessageInfo, QueryRequest, Response, StdResult, WasmQuery,
};
use quartz_dcap_verifier_msgs::QueryMsg as DcapVerifierQueryMsg;
use quartz_tcbinfo_msgs::{GetTcbInfoResponse, QueryMsg as TcbInfoQueryMsg};
use quartz_tee_ra::{
    intel_sgx::dcap::{Collateral, TrustedIdentity, TrustedMrEnclaveIdentity},
    Error as RaVerificationError,
};
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    error::Error,
    handler::Handler,
    msg::execute::attested::{
        Attestation, Attested, DcapAttestation, HasUserData, MockAttestation, MsgSansHandler, Quote,
    },
    state::CONFIG,
};

fn query_contract<T: DeserializeOwned>(
    deps: Deps<'_>,
    contract_addr: String,
    query_msg: impl Serialize,
) -> StdResult<T> {
    let request = QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr,
        msg: to_json_binary(&query_msg)?,
    });

    deps.querier.query(&request)
}

fn query_tcbinfo(deps: Deps<'_>, fmspc: String) -> Result<GetTcbInfoResponse, Error> {
    let tcbinfo_addr = {
        let config = CONFIG.load(deps.storage).map_err(Error::Std)?;
        config
            .tcbinfo_contract()
            .expect("TcbInfo contract address is required for DCAP")
            .to_string()
    };

    let fmspc_bytes =
        hex::decode(&fmspc).map_err(|_| Error::InvalidFmspc("Invalid FMSPC format".to_string()))?;
    if fmspc_bytes.len() != 6 {
        return Err(Error::InvalidFmspc("FMSPC must be 6 bytes".to_string()));
    }

    let query_msg = TcbInfoQueryMsg::GetTcbInfo { fmspc };

    query_contract(deps, tcbinfo_addr, &query_msg)
        .map_err(|err| Error::TcbInfoQueryError(err.to_string()))
}

fn to_cbor_vec<T: Serialize>(value: &T) -> Vec<u8> {
    let mut buffer = Vec::new();
    into_cbor(&value, &mut buffer).expect("Serialization failed");
    buffer
}

fn query_dcap_verifier(
    deps: Deps<'_>,
    quote: Quote,
    mr_enclave: impl Into<TrustedIdentity>,
    updated_collateral: Collateral,
) -> Result<(), Error> {
    let query_msg = DcapVerifierQueryMsg::VerifyDcapAttestation {
        quote: quote.as_ref().to_vec().into(),
        collateral: to_cbor_vec(&updated_collateral).into(),
        identities: Some(to_cbor_vec(&[mr_enclave.into()])),
    };

    let dcap_verifier_contract = {
        let config = CONFIG.load(deps.storage).map_err(Error::Std)?;
        config
            .dcap_verifier_contract()
            .expect("verifier_contract address is required for DCAP")
            .to_string()
    };

    query_contract(deps, dcap_verifier_contract, &query_msg)
        .map_err(|err| Error::DcapVerificationQueryError(err.to_string()))
}

impl Handler for DcapAttestation {
    fn handle(self, deps: DepsMut<'_>, _env: &Env, _info: &MessageInfo) -> Result<Response, Error> {
        let (quote, collateral) = self.clone().into_tuple();
        let mr_enclave = TrustedMrEnclaveIdentity::new(
            self.mr_enclave().into(),
            [""; 0],
            ["INTEL-SA-00334", "INTEL-SA-00615"],
        );

        // Retrieve the FMSPC from the collateral
        let fmspc_hex = collateral.tcb_info().to_string();

        // Query the tcbinfo contract with the FMSPC retrieved and validated
        let tcb_info_response = query_tcbinfo(deps.as_ref(), fmspc_hex)?;

        // Serialize the existing collateral
        let collateral_serialized = to_cbor_vec(&collateral);
        let mut collateral_value: CborValue = from_cbor_slice(collateral_serialized.as_slice())
            .map_err(|e| {
                Error::TcbInfoQueryError(format!("Failed to serialize collateral: {}", e))
            })?;

        // Update the tcb_info in the serialized data
        fn try_get_tcb_info(collateral_value: &mut CborValue) -> Option<&mut CborValue> {
            if let CborValue::Map(map) = collateral_value {
                return map
                    .iter_mut()
                    .find(|(k, _)| k == &CborValue::Text("tcb_info".to_string()))
                    .map(|(_, v)| v);
            }
            None
        }

        let tcb_info_value = try_get_tcb_info(&mut collateral_value).expect("infallible serde");
        *tcb_info_value = CborValue::Text(tcb_info_response.tcb_info.to_string());

        // Deserialize back into a Collateral
        let collateral_serialized = to_cbor_vec(&collateral_value);
        let updated_collateral: Collateral = from_cbor_slice(collateral_serialized.as_slice())
            .map_err(|e| {
                Error::TcbInfoQueryError(format!("Failed to deserialize updated collateral: {}", e))
            })?;

        query_dcap_verifier(deps.as_ref(), quote, mr_enclave, updated_collateral)
            .map(|_| Response::default())
    }
}

impl Handler for MockAttestation {
    fn handle(
        self,
        _deps: DepsMut<'_>,
        _env: &Env,
        _info: &MessageInfo,
    ) -> Result<Response, Error> {
        Ok(Response::default())
    }
}

impl<M, A> Handler for Attested<M, A>
where
    M: Handler + HasUserData,
    A: Handler + HasUserData + Attestation,
{
    fn handle(
        self,
        mut deps: DepsMut<'_>,
        env: &Env,
        info: &MessageInfo,
    ) -> Result<Response, Error> {
        let (msg, attestation) = self.into_tuple();
        if msg.user_data() != attestation.user_data() {
            return Err(RaVerificationError::UserDataMismatch.into());
        }

        if let Some(config) = CONFIG.may_load(deps.storage)? {
            // if we weren't able to load then the context was from InstantiateMsg so we don't fail
            // in such cases, the InstantiateMsg handler will verify that the mr_enclave matches
            if config.mr_enclave() != attestation.mr_enclave() {
                return Err(RaVerificationError::MrEnclaveMismatch.into());
            }
        }

        // handle message first, this has 2 benefits -
        // 1. we avoid (the more expensive) attestation verification if the message handler fails
        // 2. we allow the message handler to make changes to the config so that the attestation
        //    handler can use those changes, e.g. InstantiateMsg
        // return response from msg handle to include pub_key attribute
        let res_msg = Handler::handle(msg, deps.branch(), env, info)?;
        let res_attest = Handler::handle(attestation, deps, env, info)?;

        Ok(res_msg
            .add_events(res_attest.events)
            .add_attributes(res_attest.attributes))
    }
}

impl<T> Handler for MsgSansHandler<T> {
    fn handle(
        self,
        _deps: DepsMut<'_>,
        _env: &Env,
        _info: &MessageInfo,
    ) -> Result<Response, Error> {
        Ok(Response::default())
    }
}
