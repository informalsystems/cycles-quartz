use cosmwasm_schema::cw_serde;
use cosmwasm_std::StdError;
use serde::Serialize;

use crate::msg::HasDomainType;

#[derive(Clone, Debug, PartialEq)]
pub struct Sequenced<D>(pub D);

#[cw_serde]
pub struct RawSequenced<R>(pub R);

impl<R: Serialize + HasDomainType> HasDomainType for RawSequenced<R> {
    type DomainType = Sequenced<R::DomainType>;
}

impl<R: HasDomainType> TryFrom<RawSequenced<R>> for Sequenced<R::DomainType> {
    type Error = StdError;

    fn try_from(value: RawSequenced<R>) -> Result<Self, Self::Error> {
        Ok(Self(value.0.try_into()?))
    }
}

impl<R: HasDomainType> From<Sequenced<R::DomainType>> for RawSequenced<R> {
    fn from(value: Sequenced<R::DomainType>) -> Self {
        Self(value.0.into())
    }
}
