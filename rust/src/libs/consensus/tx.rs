use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------- DATA STRUCTS ----------------------
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub op: String,
    pub contract: String,
    pub function: String,
    pub args: Vec<String>,
    pub attached_symbol: Option<String>,
    pub attached_amount: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tx {
    pub signer: String,
    pub nonce: u64,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxPacked {
    pub tx_encoded: Vec<u8>,
    pub hash: Vec<u8>,
    pub signature: Vec<u8>,
    pub tx: Option<Tx>, // normalized
}

// ---------------------- ERRORS ----------------------
#[derive(Debug)]
pub enum TxError {
    TooLarge,
    NotCanonical,
    InvalidHash,
    InvalidSignature,
    NonceNotInteger,
    NonceTooHigh,
    ActionsNotList,
    ActionsLengthMustBe1,
    OpMustBeCall,
    ContractMustBeBinary,
    FunctionMustBeBinary,
    ArgsMustBeList,
    ArgMustBeBinary,
    InvalidContractOrFunction,
    InvalidModuleForSpecialMeeting,
    InvalidFunctionForSpecialMeeting,
    AttachedSymbolWrongSize,
    AttachedSymbolMustBeBinary,
    AttachedAmountMustBeBinary,
    AttachedAmountMustBeIncluded,
    Unknown,
}

pub type TxResult<T> = Result<T, TxError>;

// ---------------------- HELPERS ----------------------
fn current_time_nanos() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64
}

fn blake3_hash(_data: &[u8]) -> Vec<u8> {
    // placeholder: integrate real Blake3 hash
    vec![0u8; 32]
}

fn verify_signature(_pk: &str, _sig: &[u8], _hash: &[u8]) -> bool {
    true
}

fn validate_public_key(_pk: &str) -> bool {
    true
}

// ---------------------- TX FUNCTIONS ----------------------
impl TxPacked {
    pub fn normalize_atoms(&mut self) {
        if let Some(tx) = &mut self.tx {
            for action in &mut tx.actions {
                if action.attached_symbol.is_some() && action.attached_amount.is_none() {
                    action.attached_amount = Some("0".to_string());
                }
                if action.attached_amount.is_some() && action.attached_symbol.is_none() {
                    action.attached_symbol = Some("TOKEN".to_string());
                }
            }
        }
    }

    pub fn build(
        sk: &str,
        contract: &str,
        function: &str,
        args: Vec<String>,
        nonce: Option<u64>,
        attached_symbol: Option<String>,
        attached_amount: Option<String>,
    ) -> Self {
        let pk = sk; // placeholder for public key
        let nonce = nonce.unwrap_or_else(current_time_nanos);

        let action = Action {
            op: "call".to_string(),
            contract: contract.to_string(),
            function: function.to_string(),
            args,
            attached_symbol,
            attached_amount,
        };

        let tx_inner = Tx {
            signer: pk.to_string(),
            nonce,
            actions: vec![action],
        };

        let tx_encoded = serde_json::to_vec(&tx_inner).unwrap();
        let hash = blake3_hash(&tx_encoded);
        let signature = vec![0u8; 64]; // placeholder for signature

        Self {
            tx_encoded,
            hash,
            signature,
            tx: Some(tx_inner),
        }
    }

    pub fn pack(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap()
    }

    pub fn unpack(data: &[u8]) -> Self {
        let mut txu: TxPacked = serde_json::from_slice(data).unwrap();
        if let Some(encoded) = &txu.tx_encoded.clone().into() {
            let tx: Tx = serde_json::from_slice(encoded).unwrap();
            txu.tx = Some(tx);
        }
        txu.normalize_atoms();
        txu
    }

    pub fn validate(&mut self, is_special_meeting_block: bool) -> TxResult<()> {
        let tx_size_limit = 1024 * 1024; // placeholder

        if self.tx_encoded.len() >= tx_size_limit {
            return Err(TxError::TooLarge);
        }

        self.normalize_atoms();

        if blake3_hash(&self.tx_encoded) != self.hash {
            return Err(TxError::InvalidHash);
        }

        if !verify_signature(&self.tx.as_ref().unwrap().signer, &self.signature, &self.hash) {
            return Err(TxError::InvalidSignature);
        }

        let tx = self.tx.as_ref().unwrap();

        if tx.actions.len() != 1 {
            return Err(TxError::ActionsLengthMustBe1);
        }

        let action = &tx.actions[0];

        if action.op != "call" {
            return Err(TxError::OpMustBeCall);
        }

        if is_special_meeting_block {
            if action.contract != "Epoch" {
                return Err(TxError::InvalidModuleForSpecialMeeting);
            }
            if action.function != "slash_trainer" {
                return Err(TxError::InvalidFunctionForSpecialMeeting);
            }
        }

        if let Some(sym) = &action.attached_symbol {
            if sym.len() < 1 || sym.len() > 32 {
                return Err(TxError::AttachedSymbolWrongSize);
            }
        }

        if action.attached_symbol.is_some() && action.attached_amount.is_none() {
            return Err(TxError::AttachedAmountMustBeIncluded);
        }

        if action.attached_amount.is_some() && action.attached_symbol.is_none() {
            return Err(TxError::AttachedSymbolMustBeBinary);
        }

        Ok(())
    }
}