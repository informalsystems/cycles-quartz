use alloc::{boxed::Box, vec::Vec};
use core::fmt::Debug;

use displaydoc::Display;
use serde::{Deserialize, Serialize};
use serde_with::{hex::Hex, serde_as};
use tendermint::merkle::proof::ProofOps;
use tendermint_rpc::endpoint::abci_query::AbciQuery;

use crate::{
    error::ProofError,
    proof::{
        convert_tm_to_ics_merkle_proof,
        key::{IntoKeys, PrefixedKey},
        prefix::PrefixWasm,
        Proof,
    },
    verifier::cw::CwVerifier,
};

#[derive(Clone, Debug)]
pub struct CwProof<K = Vec<u8>, V = Vec<u8>> {
    proof: ProofOps,
    // TODO(hu55a1n1): replace `K` with `CwAbciKey`
    pub key: PrefixedKey<PrefixWasm, K>,
    pub value: V,
}

/// ABCI query response doesn't contain proof
#[derive(Clone, Debug, Display)]
pub struct ErrorWithoutProof;

impl TryFrom<AbciQuery> for CwProof {
    type Error = ErrorWithoutProof;

    fn try_from(query: AbciQuery) -> Result<Self, Self::Error> {
        RawCwProof::try_from(query).map(Into::into)
    }
}

impl<K, V> Proof for CwProof<K, V>
where
    K: Clone + Into<Vec<u8>>,
    V: AsRef<[u8]>,
{
    type Key = K;
    type Value = V;
    type ProofOps = ProofOps;

    fn verify(&self, root: Vec<u8>) -> Result<(), ProofError> {
        fn into_array_of_size_2<T: Debug>(v: Vec<T>) -> Result<[T; 2], ProofError> {
            let boxed_slice = v.into_boxed_slice();
            let boxed_array: Box<[T; 2]> = boxed_slice.try_into().expect("todo");
            Ok(*boxed_array)
        }

        let Self { proof, key, value } = self;
        let proofs = convert_tm_to_ics_merkle_proof(proof)?;

        let cw_verifier = CwVerifier::default();
        cw_verifier.verify(
            &into_array_of_size_2(proofs)?,
            &root,
            &into_array_of_size_2(key.clone().into_keys())?,
            value.as_ref(),
        )?;

        Ok(())
    }
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RawCwProof {
    #[serde_as(as = "Hex")]
    key: Vec<u8>,
    #[serde_as(as = "Hex")]
    value: Vec<u8>,
    proof: ProofOps,
}

impl RawCwProof {
    pub fn key(&self) -> &[u8] {
        self.key.as_ref()
    }
}

impl From<RawCwProof> for CwProof {
    fn from(RawCwProof { key, value, proof }: RawCwProof) -> Self {
        Self {
            proof,
            key: PrefixedKey::new(key),
            value,
        }
    }
}

impl From<CwProof> for RawCwProof {
    fn from(CwProof { proof, key, value }: CwProof) -> Self {
        Self {
            key: key.into_keys().pop().expect("empty key"),
            value,
            proof,
        }
    }
}

impl TryFrom<AbciQuery> for RawCwProof {
    type Error = ErrorWithoutProof;

    fn try_from(query: AbciQuery) -> Result<Self, Self::Error> {
        let AbciQuery {
            key, value, proof, ..
        } = query;
        let Some(proof) = proof else {
            return Err(ErrorWithoutProof);
        };

        Ok(Self { proof, key, value })
    }
}
