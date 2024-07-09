use std::{
    fs::{read, File},
    io::{Error as IoError, Write},
};

use quartz_cw::{
    msg::execute::attested::HasUserData,
    state::{MrEnclave, UserData},
};

pub trait Attestor {
    type Error: ToString;

    fn quote(&self, user_data: impl HasUserData) -> Result<Vec<u8>, Self::Error>;

    fn mr_enclave(&self) -> Result<MrEnclave, Self::Error>;
}

#[derive(Clone, PartialEq, Debug)]
pub struct EpidAttestor;

impl Attestor for EpidAttestor {
    type Error = IoError;

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
}

#[derive(Clone, PartialEq, Debug)]
pub struct MockAttestor;

impl Attestor for MockAttestor {
    type Error = String;

    fn quote(&self, user_data: impl HasUserData) -> Result<Vec<u8>, Self::Error> {
        let user_data = user_data.user_data();
        Ok(user_data.to_vec())
    }

    fn mr_enclave(&self) -> Result<MrEnclave, Self::Error> {
        Ok(Default::default())
    }
}

struct NullUserData;

impl HasUserData for NullUserData {
    fn user_data(&self) -> UserData {
        [0u8; 64]
    }
}
