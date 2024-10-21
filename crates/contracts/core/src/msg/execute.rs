pub mod attested;
pub mod sequenced;
pub mod session_create;
pub mod session_set_pub_key;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::StdError;

use crate::msg::{
    execute::{
        attested::{Attested, DefaultAttestation, RawAttested, RawDefaultAttestation},
        session_create::{RawSessionCreate, SessionCreate},
        session_set_pub_key::{RawSessionSetPubKey, SessionSetPubKey},
    },
    HasDomainType,
};

#[derive(Clone, Debug, PartialEq)]
pub enum Execute<Attestation = DefaultAttestation> {
    SessionCreate(Attested<SessionCreate, Attestation>),
    SessionSetPubKey(Attested<SessionSetPubKey, Attestation>),
}

#[cw_serde]
pub enum RawExecute<RawAttestation = RawDefaultAttestation> {
    #[serde(rename = "session_create")]
    RawSessionCreate(RawAttested<RawSessionCreate, RawAttestation>),
    #[serde(rename = "session_set_pub_key")]
    RawSessionSetPubKey(RawAttested<RawSessionSetPubKey, RawAttestation>),
}

impl<RA> TryFrom<RawExecute<RA>> for Execute<RA::DomainType>
where
    RA: HasDomainType,
{
    type Error = StdError;

    fn try_from(value: RawExecute<RA>) -> Result<Self, Self::Error> {
        match value {
            RawExecute::RawSessionCreate(msg) => {
                Ok(Execute::SessionCreate(TryFrom::try_from(msg)?))
            }
            RawExecute::RawSessionSetPubKey(msg) => {
                Ok(Execute::SessionSetPubKey(TryFrom::try_from(msg)?))
            }
        }
    }
}

impl<RA> From<Execute<RA::DomainType>> for RawExecute<RA>
where
    RA: HasDomainType,
{
    fn from(value: Execute<RA::DomainType>) -> Self {
        match value {
            Execute::SessionCreate(msg) => RawExecute::RawSessionCreate(From::from(msg)),
            Execute::SessionSetPubKey(msg) => RawExecute::RawSessionSetPubKey(From::from(msg)),
        }
    }
}

impl<RA> HasDomainType for RawExecute<RA>
where
    RA: HasDomainType,
{
    type DomainType = Execute<RA::DomainType>;
}
