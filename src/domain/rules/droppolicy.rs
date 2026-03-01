use crate::domain::entities::mir::{MirInstruction, MirOperand};
use crate::application::use_cases::mir_builder::MirBuilder;

pub struct DropPolicy;

impl DropPolicy {
    // Legacy stateful drop policy methods have been removed.
    // Drops are now strictly explicitly emitted during the AST->HIR Ownership/Borrow Checking pass
}
