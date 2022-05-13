use std::{error::Error, fmt};

use crate::component::ComponentId;

#[derive(Debug, PartialEq, Eq)]
pub struct BorrowMutError {
    component_id: ComponentId,
}

impl BorrowMutError {
    pub fn new(component_id: ComponentId) -> Self {
        Self { component_id }
    }
}

impl fmt::Display for BorrowMutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Tried to borrow component with id {:?} more than once and at least once mutably",
            self.component_id
        )
    }
}

impl Error for BorrowMutError {}
