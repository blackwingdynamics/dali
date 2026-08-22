//! Bounded Binary Metadata v2 repository verification chain.
//!
//! The implementation is kept in focused source units but included into one
//! module namespace so the bounded verification pass can share its private
//! scratch types without widening their visibility.

include!("types.rs");
include!("loading.rs");
include!("roles.rs");
include!("validation.rs");
include!("streaming.rs");
