use core::time::Duration;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{HexBinary, StdError, Uint64};
use cw_storage_plus::Item;
use k256::ecdsa::VerifyingKey;

pub type MrEnclave = [u8; 32];
pub type Nonce = [u8; 32];
pub type UserData = [u8; 64];
pub type Hash = [u8; 32];
pub type Height = u64;
pub type TrustThreshold = (u64, u64);

pub const CONFIG_KEY: &str = "quartz_config";
pub const SESSION_KEY: &str = "quartz_session";
pub const EPOCH_COUNTER_KEY: &str = "epoch_counter";
pub const CONFIG: Item<RawConfig> = Item::new(CONFIG_KEY);
pub const SESSION: Item<Session> = Item::new(SESSION_KEY);
pub const EPOCH_COUNTER: Item<Uint64> = Item::new(EPOCH_COUNTER_KEY);

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    mr_enclave: MrEnclave,
    epoch_duration: Duration,
    light_client_opts: LightClientOpts,
    tcbinfo_contract: Option<String>,
    dcap_verifier_contract: Option<String>,
}

impl Config {
    pub fn new(
        mr_enclave: MrEnclave,
        epoch_duration: Duration,
        light_client_opts: LightClientOpts,
        tcbinfo_contract: Option<String>,
        dcap_verifier_contract: Option<String>,
    ) -> Self {
        Self {
            mr_enclave,
            epoch_duration,
            light_client_opts,
            tcbinfo_contract,
            dcap_verifier_contract,
        }
    }

    pub fn light_client_opts(&self) -> &LightClientOpts {
        &self.light_client_opts
    }

    pub fn mr_enclave(&self) -> MrEnclave {
        self.mr_enclave
    }

    pub fn tcbinfo_contract(&self) -> Option<&str> {
        self.tcbinfo_contract.as_deref()
    }
}

#[cw_serde]
pub struct RawConfig {
    mr_enclave: HexBinary,
    epoch_duration: Duration,
    light_client_opts: RawLightClientOpts,
    tcbinfo_contract: Option<String>,
    dcap_verifier_contract: Option<String>,
}

impl RawConfig {
    pub fn mr_enclave(&self) -> &[u8] {
        self.mr_enclave.as_slice()
    }
    pub fn tcbinfo_contract(&self) -> Option<&str> {
        self.tcbinfo_contract.as_deref()
    }

    pub fn dcap_verifier_contract(&self) -> Option<&str> {
        self.dcap_verifier_contract.as_deref()
    }
}

impl TryFrom<RawConfig> for Config {
    type Error = StdError;

    fn try_from(value: RawConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            mr_enclave: value.mr_enclave.to_array()?,
            epoch_duration: value.epoch_duration,
            light_client_opts: value
                .light_client_opts
                .try_into()
                .map_err(|e| StdError::parse_err("light_client_opts", e))?,
            tcbinfo_contract: value.tcbinfo_contract,
            dcap_verifier_contract: value.dcap_verifier_contract,
        })
    }
}

impl From<Config> for RawConfig {
    fn from(value: Config) -> Self {
        Self {
            mr_enclave: value.mr_enclave.into(),
            epoch_duration: value.epoch_duration,
            light_client_opts: value.light_client_opts.into(),
            tcbinfo_contract: value.tcbinfo_contract,
            dcap_verifier_contract: value.dcap_verifier_contract,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LightClientOpts {
    chain_id: String,
    trusted_height: Height,
    trusted_hash: Hash,
    trust_threshold: TrustThreshold,
    trusting_period: u64,
    max_clock_drift: u64,
    max_block_lag: u64,
}

impl LightClientOpts {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        chain_id: String,
        trusted_height: Height,
        trusted_hash: Hash,
        trust_threshold: TrustThreshold,
        trusting_period: u64,
        max_clock_drift: u64,
        max_block_lag: u64,
    ) -> Result<Self, StdError> {
        let (numerator, denominator) = (trust_threshold.0, trust_threshold.1);
        if numerator > denominator {
            return Err(StdError::generic_err("trust_threshold_too_large"));
        }
        if denominator == 0 {
            return Err(StdError::generic_err("undefined_trust_threshold"));
        }
        if 3 * numerator < denominator {
            return Err(StdError::generic_err("trust_threshold_too_small"));
        }

        let _trusted_height: i64 = trusted_height
            .try_into()
            .map_err(|_| StdError::generic_err("trusted_height too large"))?;

        Ok(Self {
            chain_id,
            trusted_height,
            trusted_hash,
            trust_threshold,
            trusting_period,
            max_clock_drift,
            max_block_lag,
        })
    }

    pub fn chain_id(&self) -> &String {
        &self.chain_id
    }

    pub fn trusted_height(&self) -> Height {
        self.trusted_height
    }

    pub fn trusted_hash(&self) -> &Hash {
        &self.trusted_hash
    }

    pub fn trust_threshold(&self) -> &TrustThreshold {
        &self.trust_threshold
    }

    pub fn trusting_period(&self) -> u64 {
        self.trusting_period
    }

    pub fn max_clock_drift(&self) -> u64 {
        self.max_clock_drift
    }

    pub fn max_block_lag(&self) -> u64 {
        self.max_block_lag
    }
}

#[cw_serde]
pub struct RawLightClientOpts {
    chain_id: String,
    trusted_height: u64,
    trusted_hash: HexBinary,
    trust_threshold: (u64, u64),
    trusting_period: u64,
    max_clock_drift: u64,
    max_block_lag: u64,
}

impl TryFrom<RawLightClientOpts> for LightClientOpts {
    type Error = StdError;

    fn try_from(value: RawLightClientOpts) -> Result<Self, Self::Error> {
        Self::new(
            value.chain_id,
            value.trusted_height,
            value.trusted_hash.to_array()?,
            (value.trust_threshold.0, value.trust_threshold.1),
            value.trusting_period,
            value.max_clock_drift,
            value.max_block_lag,
        )
    }
}

impl From<LightClientOpts> for RawLightClientOpts {
    fn from(value: LightClientOpts) -> Self {
        Self {
            chain_id: value.chain_id,
            trusted_height: value.trusted_height,
            trusted_hash: Vec::<u8>::from(value.trusted_hash).into(),
            trust_threshold: (value.trust_threshold.0, value.trust_threshold.1),
            trusting_period: value.trusting_period,
            max_clock_drift: value.max_clock_drift,
            max_block_lag: value.max_block_lag,
        }
    }
}

#[cw_serde]
pub struct Session {
    nonce: HexBinary,
    pub_key: Option<HexBinary>,
}

impl Session {
    pub fn create(nonce: Nonce) -> Self {
        Self {
            nonce: nonce.into(),
            pub_key: None,
        }
    }

    pub fn with_pub_key(mut self, nonce: Nonce, pub_key: VerifyingKey) -> Option<Self> {
        if self.nonce == nonce && self.pub_key.is_none() {
            self.pub_key = Some(pub_key.to_sec1_bytes().into_vec().into());
            Some(self)
        } else {
            None
        }
    }

    pub fn nonce(&self) -> Nonce {
        self.nonce.to_array().expect("correct by construction")
    }
}
