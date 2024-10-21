use std::{
    error::Error,
    fs::{read, File},
    io::{Error as IoError, ErrorKind, Write},
};

use mc_sgx_dcap_sys_types::sgx_ql_qve_collateral_t;
use quartz_contract_core::{
    msg::{
        execute::attested::{
            Attestation, DcapAttestation, HasUserData, MockAttestation, RawDcapAttestation,
            RawMockAttestation,
        },
        HasDomainType,
    },
    state::{MrEnclave, UserData},
};
use quartz_tee_ra::intel_sgx::dcap::{Collateral, Quote3Error};
use reqwest::blocking::Client;
use serde::Serialize;

use crate::types::Fmspc;

#[cfg(not(feature = "mock-sgx"))]
pub type DefaultAttestor = DcapAttestor;

#[cfg(feature = "mock-sgx")]
pub type DefaultAttestor = MockAttestor;

const QE_IDENTITY_JSON: &str = include_str!("../data/qe_identity.json");
const ROOT_CA: &str = include_str!("../data/root_ca.pem");
const ROOT_CRL: &[u8] = include_bytes!("../data/root_crl.der");
const TCB_SIGNER: &str = include_str!("../data/tcb_signer.pem");

/// The trait defines the interface for generating attestations from within an enclave.
pub trait Attestor: Send + Sync + 'static {
    type Error: ToString;
    type Attestation: Attestation;
    type RawAttestation: HasDomainType<DomainType = Self::Attestation> + Serialize;

    fn quote(&self, user_data: impl HasUserData) -> Result<Vec<u8>, Self::Error>;

    fn mr_enclave(&self) -> Result<MrEnclave, Self::Error>;

    fn attestation(&self, user_data: impl HasUserData) -> Result<Self::Attestation, Self::Error>;
}

/// An `Attestor` for generating DCAP attestations for Gramine based enclaves.
#[derive(Clone, PartialEq, Debug)]
pub struct DcapAttestor {
    pub fmspc: Fmspc,
}

impl Attestor for DcapAttestor {
    type Error = IoError;
    type Attestation = DcapAttestation;
    type RawAttestation = RawDcapAttestation;

    fn quote(&self, user_data: impl HasUserData) -> Result<Vec<u8>, Self::Error> {
        let user_data = user_data.user_data();
        let mut user_report_data = File::create("/dev/attestation/user_report_data")?;
        user_report_data.write_all(user_data.as_slice())?;
        user_report_data.flush()?;
        read("/dev/attestation/quote")
    }

    fn mr_enclave(&self) -> Result<MrEnclave, Self::Error> {
        let quote = self.quote(NullUserData)?;
        Ok(quote[112..(112 + 32)]
            .try_into()
            .expect("hardcoded array size"))
    }

    fn attestation(&self, user_data: impl HasUserData) -> Result<Self::Attestation, Self::Error> {
        fn pccs_query_pck() -> Result<(Vec<u8>, String), Box<dyn Error>> {
            let url = "https://127.0.0.1:8081/sgx/certification/v4/pckcrl?ca=processor";

            let client = Client::builder()
                .danger_accept_invalid_certs(true) // FIXME(hu55a1n1): required?
                .build()?;
            let response = client.get(url).send()?;

            // Parse relevant headers
            let pck_crl_issuer_chain = response
                .headers()
                .get("SGX-PCK-CRL-Issuer-Chain")
                .ok_or("Missing PCK-Issuer-Chain header")?
                .to_str()?
                .to_string();

            let pck_crl = response.bytes()?;

            Ok((pck_crl.to_vec(), pck_crl_issuer_chain))
        }

        fn collateral(
            tcb_info: &str,
            pck_crl: Vec<u8>,
            pck_crl_issuer_chain: String,
        ) -> Collateral {
            let mut sgx_collateral = sgx_ql_qve_collateral_t::default();

            // SAFETY: Version is a union which is inherently unsafe
            #[allow(unsafe_code)]
            let version = unsafe { sgx_collateral.__bindgen_anon_1.__bindgen_anon_1.as_mut() };
            version.major_version = 3;
            version.minor_version = 1;

            let mut root_crl = ROOT_CRL.to_vec();
            root_crl.push(0);
            sgx_collateral.root_ca_crl = root_crl.as_ptr() as _;
            sgx_collateral.root_ca_crl_size = root_crl.len() as u32;

            let mut pck_crl = hex::decode(pck_crl).unwrap();
            pck_crl.push(0);
            sgx_collateral.pck_crl = pck_crl.as_ptr() as _;
            sgx_collateral.pck_crl_size = pck_crl.len() as u32;

            let pck_crl_issuer_chain = urlencoding::decode(&pck_crl_issuer_chain).unwrap();
            // pck_crl_issuer_chain.push(0);
            sgx_collateral.pck_crl_issuer_chain = pck_crl_issuer_chain.as_ptr() as _;
            sgx_collateral.pck_crl_issuer_chain_size = pck_crl_issuer_chain.len() as u32;

            let mut tcb_chain = [TCB_SIGNER, ROOT_CA].join("\n").as_bytes().to_vec();
            tcb_chain.push(0);
            sgx_collateral.tcb_info_issuer_chain = tcb_chain.as_ptr() as _;
            sgx_collateral.tcb_info_issuer_chain_size = tcb_chain.len() as u32;

            sgx_collateral.tcb_info = tcb_info.as_ptr() as _;
            sgx_collateral.tcb_info_size = tcb_info.len() as u32;

            // For live data the QE identity uses the same chain as the TCB info
            sgx_collateral.qe_identity_issuer_chain = tcb_chain.as_ptr() as _;
            sgx_collateral.qe_identity_issuer_chain_size = tcb_chain.len() as u32;

            sgx_collateral.qe_identity = QE_IDENTITY_JSON.as_ptr() as _;
            sgx_collateral.qe_identity_size = QE_IDENTITY_JSON.len() as u32;

            Collateral::try_from(&sgx_collateral).expect("Failed to parse collateral")
        }

        let quote = self.quote(user_data)?;

        let collateral = {
            let (pck_crl, pck_crl_issuer_chain) =
                pccs_query_pck().map_err(|e| IoError::new(ErrorKind::Other, e.to_string()))?;
            collateral(&self.fmspc.to_string(), pck_crl, pck_crl_issuer_chain)
        };

        Ok(DcapAttestation::new(
            quote
                .try_into()
                .map_err(|e: Quote3Error| IoError::other(e.to_string()))?,
            collateral,
        ))
    }
}

/// A mock `Attestor` that creates a quote consisting of just the user report data. (only meant for
/// testing purposes)
#[derive(Clone, PartialEq, Debug, Default)]
pub struct MockAttestor;

impl Attestor for MockAttestor {
    type Error = String;
    type Attestation = MockAttestation;
    type RawAttestation = RawMockAttestation;

    fn quote(&self, user_data: impl HasUserData) -> Result<Vec<u8>, Self::Error> {
        let user_data = user_data.user_data();
        Ok(user_data.to_vec())
    }

    fn mr_enclave(&self) -> Result<MrEnclave, Self::Error> {
        Ok(Default::default())
    }

    fn attestation(&self, user_data: impl HasUserData) -> Result<Self::Attestation, Self::Error> {
        Ok(MockAttestation(user_data.user_data()))
    }
}

struct NullUserData;

impl HasUserData for NullUserData {
    fn user_data(&self) -> UserData {
        [0u8; 64]
    }
}
