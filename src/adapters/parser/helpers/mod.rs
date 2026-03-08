/// Parser Helpers: Modular parsing utilities
///
/// This directory contains focused, single-responsibility helper modules used
/// by the Facade `OnuParser`.  Each module tackles one narrow concern:
///
/// - `error_recovery`: fault-tolerant synchronization strategy.

pub mod error_recovery;
