//! Invoice state machine.
//!
//! ```text
//! PENDING ──▶ PAID ──▶ SETTLED
//!    │
//!    ├──▶ FAILED     (verdict mismatch: wrong mint / amount / recipient)
//!    └──▶ EXPIRED    (TTL elapsed or poll attempts exhausted)
//! ```
//!
//! The transition table below is the *legal* set of moves. Idempotency at the
//! SOP layer is expressed as: only transition when the current status is still
//! `Pending`. Because terminal states have no outgoing edges, re-applying a
//! settlement to an already-`Paid` invoice is rejected here and treated as a
//! no-op by the caller — so a confirmation is sent exactly once.

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvoiceStatus {
    Pending,
    Paid,
    Failed,
    Expired,
    Settled,
}

impl InvoiceStatus {
    /// Canonical lowercase string used in the ledger and JSON output.
    pub fn as_str(self) -> &'static str {
        match self {
            InvoiceStatus::Pending => "pending",
            InvoiceStatus::Paid => "paid",
            InvoiceStatus::Failed => "failed",
            InvoiceStatus::Expired => "expired",
            InvoiceStatus::Settled => "settled",
        }
    }

    /// Terminal states have no outgoing transitions (except Paid → Settled).
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            InvoiceStatus::Failed | InvoiceStatus::Expired | InvoiceStatus::Settled
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct IllegalTransition {
    pub from: InvoiceStatus,
    pub to: InvoiceStatus,
}

impl fmt::Display for IllegalTransition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "illegal invoice transition {} -> {}",
            self.from.as_str(),
            self.to.as_str()
        )
    }
}

impl Error for IllegalTransition {}

/// Whether `from -> to` is a legal move in the state machine.
pub fn can_transition(from: InvoiceStatus, to: InvoiceStatus) -> bool {
    use InvoiceStatus::*;
    matches!(
        (from, to),
        (Pending, Paid) | (Pending, Failed) | (Pending, Expired) | (Paid, Settled)
    )
}

/// Guarded transition. Returns the new status, or an error if the move is not
/// legal (which the caller treats as a no-op — the core of idempotency).
pub fn transition(
    from: InvoiceStatus,
    to: InvoiceStatus,
) -> Result<InvoiceStatus, IllegalTransition> {
    if can_transition(from, to) {
        Ok(to)
    } else {
        Err(IllegalTransition { from, to })
    }
}

#[cfg(test)]
mod tests {
    use super::InvoiceStatus::*;
    use super::*;

    #[test]
    fn legal_transitions() {
        assert!(can_transition(Pending, Paid));
        assert!(can_transition(Pending, Failed));
        assert!(can_transition(Pending, Expired));
        assert!(can_transition(Paid, Settled));
    }

    #[test]
    fn re_settling_a_paid_invoice_is_illegal() {
        // This is what makes double-confirmation impossible: once Paid, a second
        // verify tick that tries Pending->Paid finds current == Paid and the
        // move Paid->Paid is not legal, so the caller no-ops.
        assert!(!can_transition(Paid, Paid));
        assert_eq!(
            transition(Paid, Paid),
            Err(IllegalTransition {
                from: Paid,
                to: Paid
            })
        );
    }

    #[test]
    fn terminal_states_are_frozen() {
        for from in [Failed, Expired, Settled] {
            for to in [Pending, Paid, Failed, Expired, Settled] {
                assert!(
                    !can_transition(from, to),
                    "{from:?} -> {to:?} must be illegal"
                );
            }
        }
    }

    #[test]
    fn cannot_skip_states() {
        assert!(!can_transition(Pending, Settled)); // must go via Paid
        assert!(!can_transition(Paid, Failed));
        assert!(!can_transition(Paid, Expired));
    }

    #[test]
    fn status_strings_are_stable() {
        assert_eq!(Pending.as_str(), "pending");
        assert_eq!(Paid.as_str(), "paid");
        assert_eq!(Settled.as_str(), "settled");
    }
}
