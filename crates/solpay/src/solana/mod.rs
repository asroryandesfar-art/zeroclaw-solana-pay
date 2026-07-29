//! Solana adapter layer.
//!
//! Hard split between **fetching** (`rpc`, lands next) and **deciding**
//! (`verify`, lands next): the payment verdict is a pure function of already
//! fetched data so it can be tested offline against fixtures. This module holds
//! the deterministic building blocks: key parsing, ATA derivation, reference
//! generation, and the Solana Pay URL.

pub mod ata;
pub mod commitment;
pub mod model;
pub mod pay_url;
pub mod pubkey;
pub mod reference;
pub mod rpc;
pub mod verify;

pub use ata::associated_token_address;
pub use commitment::{CommitmentLevel, ParseCommitmentError};
pub use model::{AccountLamportDelta, SignatureRecord, TokenBalanceDelta, TransactionEvidence};
pub use pay_url::{build_transfer_request_url, TransferRequest};
pub use pubkey::{parse_merchant_wallet, parse_pubkey, PubkeyError};
pub use reference::generate as generate_reference;
pub use rpc::{
    parse_signatures_response, parse_transaction_response, HttpTransport, RpcClient, RpcError,
    TransportError, UreqTransport,
};
pub use verify::{
    decide, evaluate, verify_payment, Candidate, Expected, ExpectedAsset, TxOutcome, Verdict,
};
