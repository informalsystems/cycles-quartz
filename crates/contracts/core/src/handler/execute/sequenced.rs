use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, StdResult, Uint64};

use crate::{
    error::Error, handler::Handler, msg::execute::sequenced::SequencedMsg, state::SEQUENCE_NUM,
};

impl<T: Handler> Handler for SequencedMsg<T> {
    fn handle(self, deps: DepsMut<'_>, env: &Env, info: &MessageInfo) -> Result<Response, Error> {
        SEQUENCE_NUM.update(deps.storage, |mut counter| -> StdResult<_> {
            counter += Uint64::one();
            Ok(counter)
        })?;

        self.0.handle(deps, env, info)
    }
}
