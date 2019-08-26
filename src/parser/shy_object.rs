
use std::rc::Rc;
use std::cell::RefCell;

use super::shy_association::ShyAssociation;

pub struct ShyObject {
    pub association: Rc<RefCell<ShyAssociation>>
}

impl Clone for ShyObject {
    fn clone(&self) -> Self {
        ShyObject {
            association: self.association.borrow().clone_association()
        }
    }
}

impl PartialEq for ShyObject {
    fn eq(&self, other: &ShyObject) -> bool {
        let other_assoc: &ShyAssociation = &*other.association.borrow();
        self.association.borrow().equals_association(other_assoc)
    }
}
