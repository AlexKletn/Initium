use super::*;

pub(crate) fn assert_already_initialized() {
    assert!(matches!(initialize(), Err(Error::Reentrant)));
}
