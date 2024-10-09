use alloc::vec::Vec;
use core::marker::PhantomData;

use ics23::{
    calculate_existence_root, commitment_proof::Proof, verify_membership, CommitmentProof,
    ProofSpec,
};

use crate::{error::ProofError, verifier::Verifier};

#[derive(Clone, Debug)]
pub struct Ics23MembershipVerifier<K, V> {
    spec: ProofSpec,
    _phantom: PhantomData<(K, V)>,
}

impl<K, V> Ics23MembershipVerifier<K, V> {
    pub fn new(spec: ProofSpec) -> Self {
        Self {
            spec,
            _phantom: Default::default(),
        }
    }
}

impl<K, V> Verifier for Ics23MembershipVerifier<K, V>
where
    K: AsRef<[u8]>,
    V: AsRef<[u8]>,
{
    type Proof = CommitmentProof;
    type Root = Vec<u8>;
    type Key = K;
    type Value = V;
    type Error = ProofError;

    fn verify(
        &self,
        commitment_proof: &Self::Proof,
        key: &Self::Key,
        value: &Self::Value,
    ) -> Result<Self::Root, Self::Error> {
        if value.as_ref().is_empty() {
            return Err(ProofError::EmptyVerifiedValue);
        }

        let Some(Proof::Exist(existence_proof)) = &commitment_proof.proof else {
            return Err(ProofError::InvalidMerkleProof);
        };

        let root = calculate_existence_root::<ics23::HostFunctionsManager>(existence_proof)
            .map_err(|_| ProofError::InvalidMerkleProof)?;

        if !verify_membership::<ics23::HostFunctionsManager>(
            commitment_proof,
            &self.spec,
            &root,
            key.as_ref(),
            value.as_ref(),
        ) {
            return Err(ProofError::VerificationFailure);
        }

        Ok(root)
    }
}
