use std::str::FromStr;

use anyhow::anyhow;
use bip32::secp256k1::{
    ecdsa::VerifyingKey,
    sha2::{Digest, Sha256},
};
use clap::Parser;
use cosmrs::{tendermint::chain::Id as TmChainId, AccountId};
use cosmwasm_std::{Addr, HexBinary};
use cw_tee_mtcs::state::{LiquiditySource, LiquiditySourceType};
use mtcs_enclave::types::{
    ContractObligation, RawEncryptedObligation, RawSetOff, SubmitObligationsMsg,
    SubmitObligationsMsgInner,
};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use subtle_encoding::bech32::decode as bech32_decode;
use wasmd_client::{CliWasmdClient, WasmdClient};

// const MNEMONIC_PHRASE: &str = "clutch debate vintage foster barely primary clown leader sell manual leopard ladder wet must embody story oyster imitate cable alien six square rice wedding";

const ADDRESS_PREFIX: &str = "neutron";

type Sha256Digest = [u8; 32];

#[derive(Clone, Debug, Serialize, Deserialize)]
struct QueryAllSetoffsResponse {
    setoffs: Vec<(HexBinary, RawSetOff)>,
}

#[derive(Clone, Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, value_parser = wasmaddr_to_id)]
    mtcs: AccountId,

    #[arg(short, long)]
    epoch_pk: String,

    #[arg(short, long)]
    overdraft: String,

    #[clap(long)]
    flip: bool,

    #[arg(
        short,
        long,
        default_value = "wasm14qdftsfk6fwn40l0xmruga08xlczl4g05npy70"
    )]
    admin: String,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    let mut alice = Addr::unchecked("wasm124tuy67a9dcvfgcr4gjmz60syd8ddaugl33v0n");
    let mut bob = Addr::unchecked("wasm1ctkqmg45u85jnf5ur9796h7ze4hj6ep5y7m7l6");
    let overdraft = Addr::unchecked(cli.overdraft);

    if cli.flip {
        let temp = alice.clone();
        alice = bob;
        bob = temp;
    }

    let alice_to_bob: ContractObligation = ContractObligation {
        debtor: alice.clone(),
        creditor: bob.clone(),
        amount: 10,
        salt: HexBinary::from([0; 64]),
    };

    let bob_acceptance: ContractObligation = ContractObligation {
        debtor: bob.clone(),
        creditor: overdraft.clone(),
        amount: 10,
        salt: HexBinary::from([0; 64]),
    };

    let alice_tender: ContractObligation = ContractObligation {
        debtor: overdraft.clone(),
        creditor: alice.clone(),
        amount: 10,
        salt: HexBinary::from([0; 64]),
    };

    let intents = vec![alice_to_bob, bob_acceptance, alice_tender];
    let epoch_pk = VerifyingKey::from_sec1_bytes(&hex::decode(cli.epoch_pk).unwrap()).unwrap();

    let intents_enc = encrypt_overdraft_intents(intents, &epoch_pk);

    let liquidity_sources: Vec<LiquiditySource> = vec![LiquiditySource {
        address: overdraft,
        source_type: LiquiditySourceType::Overdraft,
    }];

    let msg: &str = &create_wasm_msg(intents_enc, liquidity_sources)
        .expect("cannot parse")
        .to_string();

    let node_url = Url::parse("http://143.244.186.205:26657")?;
    let chain_id = TmChainId::from_str("testing")?;

    let wasmd_client = CliWasmdClient::new(node_url);

    wasmd_client.tx_execute(
        &cli.mtcs,
        &chain_id,
        4000000,
        &cli.admin.to_string(),
        "50000untrn",
        msg,
    )?;

    Ok(())
}

pub struct OverdraftObligation {
    pub debtor: Addr,
    pub creditor: Addr,
    pub amount: u64,
}

fn encrypt_overdraft_intents(
    intents: Vec<ContractObligation>,
    epoch_pk: &VerifyingKey,
) -> Vec<(Sha256Digest, Vec<u8>)> {
    let mut intents_enc = vec![];

    for i in intents {
        // serialize intent
        let i_ser = serde_json::to_string(&i).unwrap();

        // encrypt intent
        let i_cipher = ecies::encrypt(&epoch_pk.to_sec1_bytes(), i_ser.as_bytes()).unwrap();

        // hash intent
        let i_digest: Sha256Digest = {
            let mut hasher = Sha256::new();
            hasher.update(i_ser);
            hasher.finalize().into()
        };

        intents_enc.push((i_digest, i_cipher));
    }

    intents_enc
}

fn create_wasm_msg(
    obligations_enc: Vec<(Sha256Digest, Vec<u8>)>,
    liquidity_sources: Vec<LiquiditySource>,
) -> anyhow::Result<serde_json::Value> {
    let obligations_enc: Vec<_> = obligations_enc
        .into_iter()
        .map(|(digest, ciphertext)| {
            let digest = HexBinary::from(digest);
            let ciphertext = HexBinary::from(ciphertext);
            RawEncryptedObligation { digest, ciphertext }
        })
        .collect();

    let msg = SubmitObligationsMsg {
        submit_obligations: SubmitObligationsMsgInner {
            obligations: obligations_enc,
            liquidity_sources,
        },
    };
    serde_json::to_value(msg).map_err(Into::into)
}

fn wasmaddr_to_id(address_str: &str) -> anyhow::Result<AccountId> {
    let (hr, _) = bech32_decode(address_str).map_err(|e| anyhow!(e))?;
    if hr != ADDRESS_PREFIX {
        return Err(anyhow!(hr));
    }

    Ok(address_str.parse().unwrap())
}
