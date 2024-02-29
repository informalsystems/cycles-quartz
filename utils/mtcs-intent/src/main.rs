#![forbid(unsafe_code)]
#![warn(
    clippy::checked_conversions,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used,
    trivial_casts,
    trivial_numeric_casts,
    rust_2018_idioms,
    unused_lifetimes,
    unused_import_braces,
    unused_qualifications
)]

use std::{
    error::Error,
    fs::{read_to_string, File},
    io::Write,
    path::PathBuf,
};

use clap::{Parser, Subcommand};
use cosmwasm_std::HexBinary;
use ecies::encrypt;
use k256::ecdsa::{SigningKey, VerifyingKey};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
enum Command {
    KeyGen {
        #[clap(long, default_value = "user.pk")]
        pk_file: PathBuf,
        #[clap(long, default_value = "user.sk")]
        sk_file: PathBuf,
    },
    EncryptObligation {
        #[clap(long, value_parser = parse_obligation_json)]
        obligation: Obligation,
        #[clap(long, default_value = "epoch.pk")]
        pk_file: PathBuf,
    },
}

fn parse_obligation_json(s: &str) -> Result<Obligation, String> {
    let raw_obligation: RawObligation = serde_json::from_str(s).map_err(|e| e.to_string())?;
    raw_obligation.try_into()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RawObligation {
    debtor: HexBinary,
    creditor: HexBinary,
    amount: u64,
    #[serde(default)]
    salt: HexBinary,
}

#[derive(Clone, Debug)]
struct Obligation {
    debtor: VerifyingKey,
    creditor: VerifyingKey,
    amount: u64,
    salt: [u8; 64],
}

impl TryFrom<RawObligation> for Obligation {
    type Error = String;

    fn try_from(raw_obligation: RawObligation) -> Result<Self, Self::Error> {
        let mut salt = [0u8; 64];
        rand::thread_rng().fill(&mut salt[..]);

        Ok(Self {
            debtor: VerifyingKey::from_sec1_bytes(raw_obligation.debtor.as_slice())
                .map_err(|e| e.to_string())?,
            creditor: VerifyingKey::from_sec1_bytes(raw_obligation.creditor.as_slice())
                .map_err(|e| e.to_string())?,
            amount: raw_obligation.amount,
            salt,
        })
    }
}

impl From<Obligation> for RawObligation {
    fn from(obligation: Obligation) -> Self {
        Self {
            debtor: obligation.debtor.to_sec1_bytes().into_vec().into(),
            creditor: obligation.creditor.to_sec1_bytes().into_vec().into(),
            amount: obligation.amount,
            salt: obligation.salt.into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct EncryptedObligation {
    ciphertext: HexBinary,
    digest: HexBinary,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();

    match args.command {
        Command::KeyGen { pk_file, sk_file } => {
            let sk = SigningKey::random(&mut rand::thread_rng());
            let pk = sk.verifying_key();

            let mut sk_file = File::create(sk_file)?;
            let sk = hex::encode(sk.to_bytes());
            sk_file.write_all(sk.as_bytes())?;

            let mut pk_file = File::create(pk_file)?;
            let pk = hex::encode(pk.to_sec1_bytes());
            pk_file.write_all(pk.as_bytes())?;
        }
        Command::EncryptObligation {
            obligation,
            pk_file,
        } => {
            let epoch_pk = {
                let pk_str = read_to_string(pk_file)?;
                hex::decode(pk_str)?
            };
            let obligation_ser = serde_json::to_string(&RawObligation::from(obligation))
                .expect("infallible serializer");

            let ciphertext =
                encrypt(&epoch_pk, obligation_ser.as_bytes()).map_err(|e| e.to_string())?;

            let digest: [u8; 32] = {
                let mut hasher = Sha256::new();
                hasher.update(obligation_ser);
                hasher.finalize().into()
            };

            let obligation_enc = EncryptedObligation {
                ciphertext: ciphertext.into(),
                digest: digest.into(),
            };

            println!(
                "{}",
                serde_json::to_string(&obligation_enc).expect("infallible serializer")
            );
        }
    }

    Ok(())
}
