#![cfg_attr(not(feature = "std"), no_std)]

use sp_core::{ecdsa, H160, H256, RuntimeDebug};
use sp_runtime::traits::{IdentifyAccount, Verify, Lazy};
use sha3::{Digest, Keccak256};
use codec::{Encode, Decode};

#[cfg(feature = "std")]
pub use serde::{Serialize, Deserialize, de::DeserializeOwned};

/// Signature verify that can work with Ethereum signature types..
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Eq, PartialEq, Clone, Encode, Decode, RuntimeDebug)]
pub struct Signature(ecdsa::Signature);

impl From<ecdsa::Signature> for Signature {
    fn from(x: ecdsa::Signature) -> Self {
        Signature(x)
    }
}

/// Public key for Ethereum
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Signer([u8; 20]);

impl IdentifyAccount for Signer {
    type AccountId = H160;
    fn into_account(self) -> H160 {
        self.0.into()
    }
}

impl From<[u8; 20]> for Signer {
    fn from(x: [u8; 20]) -> Self {
        Signer(x)
    }
}

impl From<ecdsa::Public> for Signer {
    fn from(x: ecdsa::Public) -> Self {
        let mut m = [0u8; 20];
        m.copy_from_slice(&x.as_ref()[13..33]);
        Signer(m)
    }
}

impl Verify for Signature {
    type Signer = Signer;
    fn verify<L: Lazy<[u8]>>(&self, mut msg: L, signer: &H160) -> bool {
        let mut m = [0u8; 32];
        m.copy_from_slice(Keccak256::digest(msg.get()).as_slice());

        match sp_io::crypto::secp256k1_ecdsa_recover(self.0.as_ref(), &m) {
            Ok(pubkey) => {
                let verified = H160::from(H256::from_slice(Keccak256::digest(&pubkey).as_slice()));
                println!("verified {:?}", verified);
                println!("signer {:?}", *signer);
                verified == *signer
            }
            _ => {
                log::error!("Invalid signature");
                false
            },
        }
    }
}



#[cfg(feature = "std")]
impl std::fmt::Display for Signer {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "Ethereum signature: {:?}", H160::from_slice(&self.0))
    }
}

