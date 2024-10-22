use cosmwasm_std::{DepsMut, Env, HexBinary, MessageInfo, Response, Uint64};

use crate::{
    error::Error,
    handler::Handler,
    msg::execute::session_set_pub_key::SessionSetPubKey,
    state::{SEQUENCE_NUM, SESSION},
};

impl Handler for SessionSetPubKey {
    fn handle(self, deps: DepsMut<'_>, _env: &Env, _info: &MessageInfo) -> Result<Response, Error> {
        let session = SESSION.load(deps.storage).map_err(Error::Std)?;
        let (nonce, pub_key) = self.into_tuple();

        let session = session
            .with_pub_key(nonce, pub_key)
            .ok_or(Error::BadSessionTransition)?;
        SESSION.save(deps.storage, &session).map_err(Error::Std)?;

        let sequence_num = Uint64::new(0);
        SEQUENCE_NUM
            .save(deps.storage, &sequence_num)
            .map_err(Error::Std)?;

        Ok(Response::new()
            .add_attribute("action", "session_set_pub_key")
            .add_attribute(
                "pub_key",
                HexBinary::from(pub_key.to_sec1_bytes().into_vec()).to_hex(),
            ))
    }
}
