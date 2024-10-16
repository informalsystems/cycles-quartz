pub mod attested;
pub mod sequenced;
pub mod session_create;
pub mod session_set_pub_key;

use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

use crate::{
    error::Error,
    handler::Handler,
    msg::execute::{
        attested::{Attestation, HasUserData},
        Execute,
    },
};

impl<A> Handler for Execute<A>
where
    A: Handler + HasUserData + Attestation,
{
    fn handle(self, deps: DepsMut<'_>, env: &Env, info: &MessageInfo) -> Result<Response, Error> {
        match self {
            Execute::SessionCreate(msg) => msg.handle(deps, env, info),
            Execute::SessionSetPubKey(msg) => msg.handle(deps, env, info),
        }
    }
}
