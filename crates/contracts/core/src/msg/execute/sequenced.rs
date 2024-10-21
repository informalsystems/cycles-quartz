use cosmwasm_schema::cw_serde;
use cosmwasm_std::StdError;
use serde::Serialize;

use crate::msg::HasDomainType;

#[derive(Clone, Debug, PartialEq)]
pub struct SequencedMsg<D>(pub D);

#[cw_serde]
pub struct RawSequencedMsg<R>(pub R);

impl<R: Serialize + HasDomainType> HasDomainType for RawSequencedMsg<R> {
    type DomainType = SequencedMsg<R::DomainType>;
}

impl<R: HasDomainType> TryFrom<RawSequencedMsg<R>> for SequencedMsg<R::DomainType> {
    type Error = StdError;

    fn try_from(value: RawSequencedMsg<R>) -> Result<Self, Self::Error> {
        Ok(Self(value.0.try_into()?))
    }
}

impl<R: HasDomainType> From<SequencedMsg<R::DomainType>> for RawSequencedMsg<R> {
    fn from(value: SequencedMsg<R::DomainType>) -> Self {
        Self(value.0.into())
    }
}
