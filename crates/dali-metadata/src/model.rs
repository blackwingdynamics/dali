//! Fixed-capacity metadata contract records.

mod bundle;
mod core;
mod delegation;
mod repository;
mod revocation;
mod snapshot;
mod targets;
mod trust_store;

pub use bundle::*;
pub use core::*;
pub use delegation::*;
pub use repository::*;
pub use revocation::*;
pub use snapshot::*;
pub use targets::*;
pub use trust_store::TrustStorePayload;

#[cfg(test)]
mod tests {
    use super::{BoundedText, TextError};

    #[test]
    fn stores_bounded_text_without_heap_allocation() {
        let value = BoundedText::<8>::new("dali").expect("test value fits");
        assert_eq!(value.as_str(), Some("dali"));
    }

    #[test]
    fn rejects_empty_and_oversized_text() {
        assert_eq!(BoundedText::<8>::new(""), Err(TextError::Empty));
        assert_eq!(BoundedText::<3>::new("dali"), Err(TextError::TooLong));
    }
}
