//! Domain layer: pure business rules with no I/O and no network. Fully
//! unit-testable in isolation.

pub mod invoice;
pub mod validation;

pub use invoice::{can_transition, transition, IllegalTransition, InvoiceStatus};
pub use validation::{validate_amount, validate_token, ChargeLimits, ValidationError};
