// Capabilities
//================

/*
Defines a capability-based permission system for Mech, featuring an enum Capability representing different system permissions, and a struct CapabilityToken storing capability tokens with their associated permissions. The CapabilityToken methods allow creating, signing, verifying, and revoking tokens. Additionally, the generate_keypair function generates cryptographic keypairs for signing and verifying tokens, ensuring secure access management for the file system.
*/

use ed25519_dalek::{Verifier, Signer, Keypair, PublicKey, Signature};
use rand::rngs::OsRng;
use rand::RngCore;
use crate::hash_str;
use hashbrown::HashSet;
use crate::*;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Capability {
  PinCPUCore(u64) ,                // Pin Mech program to a particular CPU core
  AvailableCores(u64),             // 0 indicats all, otherwise the number of CPU cores to use is specified
  GPGPURead,                       // Permission to read from GPGPU
  GPGPUWrite,                      // Permission to write to GPGPU 
  InputArguments,                  // Permission to accept input arguments
  StdOut,                          // Permission to write to stdout stream
  StdIn,                           // Permission to read from stdin stream
  StdErr,                          // Permission to write to stderr stream
  CoreDatabaseRead,                // Read access to the database of a core
  CoreDatabaseWrite,               // Write access to the database of a core. Cores without this permission are read-only data sources
  DownloadDependencies,            // Permission for the program to download dependencies
  LoadDependencies,                // Permission for the program to load dependencies from disk
  CoreNetworkRead,                 // Read access for a core node to receive messages from other cores
  CoreNetworkWrite,                // Write access for a core node to send messages to other cores
  NetworkRead,                     // Read access to the general network interface
  NetworkWrite,                    // Write access to the general network interface
  FileSystemRead,                  // Read access to the whole file system
  FileSystemWrite,                 // Write access to the whole file system
  FileRead(String),                // Read access to a specific file or folder
  FileWrite(String),               // Write access to a specific file or folder
  AllTablesRead,                   // Allow all tables to be read. If you don't include this capability, you should include per-table read permissions
  AllTablesWrite,                  // Allow all tables to be written. If you don't include this capability, you should include per-table write permissions
  TableRead((TableId,u64,u64)),    // Read access to a specific table{row,col}
  TableWrite((TableId,u64,u64)),   // Write access to a specific table{row,col}
  UserDefined(u64),                // Users can define their own custom capabilities with an id
}

#[derive(Clone)]
pub struct CapabilityToken {
  id: u64,
  name: String,
  capabilities: HashSet<Capability>,
  owner: u64,
  expires: Option<u64>,
  signature: Option<(Signature,PublicKey)>, // WARNING: Including the public key with the signature makes it vulnerable to MITM attacks. Use secure channels where security is necessary.
}

impl CapabilityToken {

  // Create a new CapabilityToken with the given name, capabilities, owner, and expiration time
  pub fn new(
    name: String,
    capabilities: HashSet<Capability>,
    owner: u64,
    expires: Option<u64>) -> CapabilityToken {
    let data = format!("{:?}{:?}{:?}", &name, &owner, &capabilities);
    let id = hash_str(&data);
    CapabilityToken {
      id,
      name,
      capabilities,
      owner,
      expires,
      signature: None,
    }
  }

  // Sign the token using a provided keypair
  pub fn sign(&mut self, keypair: &Keypair) -> Result<(),MechError> {
    match self.signature {
      Some(s) => { Err(MechError{msg: "".to_string(), id: 3295, kind: MechErrorKind::GenericError(format!("Capability already signed"))})},
      None => {
        let data_str = format!("{:?}{:?}{:?}", &self.name, &self.owner, &self.capabilities);
        let data_bytes = data_str.as_bytes();        
        let signature = keypair.sign(&data_bytes);
        self.signature = Some((signature,keypair.public));
        Ok(())
      }
    }
  }

  // Verify the token's signature using a provided public key
  pub fn verify(&self) -> Result<(),MechError> {
    match self.signature {
      Some((s,public_key)) => {
        let data_str = format!("{:?}{:?}{:?}", &self.name, &self.owner, &self.capabilities);
        let data_bytes = data_str.as_bytes();
        if public_key.verify(&data_bytes, &s).is_ok() {
          Ok(())
        } else {
          Err(MechError{id: 83820, msg: "".to_string(), kind: MechErrorKind::InvalidCapabilityToken})
        }
      },
      None => Err(MechError{id: 83821, msg: "".to_string(), kind: MechErrorKind::InvalidCapabilityToken})
    }
  }

  // Check to see if a token has a given capability
  pub fn has_capability(&self, capability: &Capability) -> Result<(),MechError> {
    if self.capabilities.contains(capability) {
      Ok(())
    } else {
      Err(MechError{id: 83822, msg: "".to_string(), kind: MechErrorKind::MissingCapability(capability.clone())})
    }
  }

  // Returns true if the token is valid and contains the capability, false otherwise.
  pub fn verify_capability(&self, capability: &Capability) -> Result<(),MechError> {
    match self.verify() {
      Ok(()) => self.has_capability(capability),
      x => x,
    }
  }

  // Revoke the token by removing its expiration time and signature, so it cannot be validated
  pub fn revoke(&mut self) {
    self.expires = None;
    self.signature = None;
  }

}

impl fmt::Debug for CapabilityToken {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    for c in self.capabilities.iter() {
      write!(f,"{:?}\n",c)?;
    }
    Ok(())
  }
}

// Generate a new id for creating unique owner ids
pub fn generate_uuid() -> u64 {
  let mut key = [0u8; 16];
  OsRng.fill_bytes(&mut key);
  OsRng.next_u64()
}


// Generate a new keypair for signing and verifying tokens
pub fn generate_keypair() -> Keypair {
  let mut csprng = OsRng{};
  Keypair::generate(&mut csprng)
}