
use std::rc::Rc;
use std::cell::RefCell;
use std::fmt::*;
use std::ops::{Deref, DerefMut};
use std::collections::HashMap;

use super::shy_association::*;
use super::shy_token::ShyValue;
use super::shy_scalar::ShyScalar;

/// A Holder for a ShyAssociation that permits the implementation of Clone, PartialEq, and Debug for a Trait object.
pub struct ShyObject {
    pub association: Rc<RefCell<dyn ShyAssociation>>
}

impl ShyObject {
    /// Create a new ShyObject that contains a clone of the given ShyAssociation.
    pub fn new<A : ShyAssociation>(value: &A) -> ShyObject {
        ShyObject {
            association: value.clone_association()
        }
    }

    /// Create a new ShyObject that contains a clone of the given Rc, ensuring that the underlying ShyAssociation is NOT cloned.
    pub fn share(wrapped: Rc<RefCell<dyn ShyAssociation>>) -> ShyObject {
        ShyObject {
            association: wrapped.clone()
        }
    }

    /// Create a shallow clone of this ShyObject that has a new Rc that points to the same RefCell and hence the same ShyAssociation.
    pub fn shallow_clone(&self) -> ShyObject {
        ShyObject {
            association: self.association.clone()
        }
    }

    fn as_deref(&self) -> impl Deref<Target = dyn ShyAssociation> {
        self.association.borrow()
    }   
    fn as_deref_mut(&self) -> impl DerefMut<Target = dyn ShyAssociation> {
        self.association.borrow_mut()
    }
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

impl Debug for ShyObject {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.association.borrow().to_indented_string(2, 2))
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[allow(unused_imports)]
    use spectral::prelude::*;

    #[test]
    fn as_deref() {
        let mut dictionary1 : HashMap<&str, ShyValue> = HashMap::new();
        set_into(&mut dictionary1, "name", "The Doctor");
        set_into(&mut dictionary1, "season", 12);
        set_into(&mut dictionary1, "popular", true);
        let wrapped: Rc<RefCell<dyn ShyAssociation>> = Rc::new(RefCell::new(dictionary1));
        let shy_obj = ShyObject::share(wrapped);
        let deref = shy_obj.as_deref();
        let actual = deref.get("name");
        let expected = ShyValue::Scalar(ShyScalar::String("The Doctor".to_owned()));
        assert_eq!(Option::Some(&expected), actual);
    }

}
