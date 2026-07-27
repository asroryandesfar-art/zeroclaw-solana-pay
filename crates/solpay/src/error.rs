//! Error taxonomy and the deterministic exit-code contract.
//!
//! ZeroClaw SOPs branch on `solpay`'s exit code, so the mapping is a stable API:
//!
//! | Code | Meaning        | SOP reaction                                    |
//! |------|----------------|-------------------------------------------------|
//! | 0    | success        | read the JSON on stdout                         |
//! | 2    | invalid input  | reject to the user (bad amount/reference/token) |
//! | 3    | config error   | halt; operator misconfiguration                 |
//! | 4    | RPC / transient| keep the invoice PENDING; retry next tick       |
//! | 5    | internal error | alert; leave state untouched                    |
//!
//! Rule of thumb: a value that came from a person/message maps to 2; a value
//! that came from locked config/environment maps to 3; an unreachable/unknown
//! chain maps to 4; anything genuinely unexpected maps to 5.

use std::fmt;

use crate::config::ConfigError;
use crate::domain::validation::ValidationError;
use crate::money::MoneyError;
use crate::solana::pubkey::PubkeyError;
use crate::solana::rpc::RpcError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Success = 0,
    InvalidInput = 2,
    Config = 3,
    RpcTransient = 4,
    Internal = 5,
}

impl ExitCode {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

/// A CLI error carrying both a deterministic exit code and a human message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppError {
    pub code: ExitCode,
    pub message: String,
}

impl AppError {
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self {
            code: ExitCode::InvalidInput,
            message: message.into(),
        }
    }
    pub fn config(message: impl Into<String>) -> Self {
        Self {
            code: ExitCode::Config,
            message: message.into(),
        }
    }
    pub fn rpc(message: impl Into<String>) -> Self {
        Self {
            code: ExitCode::RpcTransient,
            message: message.into(),
        }
    }
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            code: ExitCode::Internal,
            message: message.into(),
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AppError {}

// --- conversions preserve the correct exit code ---------------------------

impl From<ConfigError> for AppError {
    fn from(e: ConfigError) -> Self {
        AppError::config(e.to_string())
    }
}

impl From<RpcError> for AppError {
    fn from(e: RpcError) -> Self {
        AppError::rpc(e.to_string())
    }
}

impl From<MoneyError> for AppError {
    fn from(e: MoneyError) -> Self {
        AppError::invalid_input(e.to_string())
    }
}

impl From<ValidationError> for AppError {
    fn from(e: ValidationError) -> Self {
        AppError::invalid_input(e.to_string())
    }
}

impl From<PubkeyError> for AppError {
    fn from(e: PubkeyError) -> Self {
        // A bad public key given as a command argument is invalid input; when it
        // comes from config it is wrapped as ConfigError before reaching here.
        AppError::invalid_input(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_are_stable() {
        assert_eq!(ExitCode::Success.as_i32(), 0);
        assert_eq!(ExitCode::InvalidInput.as_i32(), 2);
        assert_eq!(ExitCode::Config.as_i32(), 3);
        assert_eq!(ExitCode::RpcTransient.as_i32(), 4);
        assert_eq!(ExitCode::Internal.as_i32(), 5);
    }

    #[test]
    fn conversions_preserve_codes() {
        let c: AppError = ConfigError::MainnetNotAllowed.into();
        assert_eq!(c.code, ExitCode::Config);

        let r: AppError = RpcError::Unavailable.into();
        assert_eq!(r.code, ExitCode::RpcTransient);

        let m: AppError = MoneyError::Empty.into();
        assert_eq!(m.code, ExitCode::InvalidInput);

        let v: AppError = ValidationError::ZeroAmount.into();
        assert_eq!(v.code, ExitCode::InvalidInput);
    }
}
