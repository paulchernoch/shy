
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

    pub fn empty() -> ShyObject {
        ShyObject {
            association: Rc::new(RefCell::new(HashMap::new()))
        }
    }

    /// Create a shallow clone of this ShyObject that has a new Rc that points to the same RefCell and hence the same ShyAssociation.
    pub fn shallow_clone(&self) -> ShyObject {
        ShyObject {
            association: self.association.clone()
        }
    }

    pub fn as_deref(&self) -> impl Deref<Target = dyn ShyAssociation> {
        self.association.borrow()
    }

    pub fn as_deref_mut(&self) -> impl DerefMut<Target = dyn ShyAssociation> {
        self.association.borrow_mut()
    }

    /// Follow a path from the given ShyObject down to one of its descendants and retrieve that descendant as an Option.
    /// If any levels of the hierarchy are missing and can be added, add them using the supplied generator.
    /// If at any stage it is not permitted to add or follow a given property in tht path, return None.
    /// If the path vector is empty, return a shallow clone of self.
    pub fn vivify<'a>(&'a mut self, path: Vec<&'static str>, generator: impl Fn() -> ShyObject) -> Option<ShyObject> {
        if path.len() == 0 {
            return Some(self.shallow_clone())
        }
        let mut current_object = self.shallow_clone();
        let mut next_object;
        for key in path {
            {
                let mut deref = current_object.as_deref_mut();
                if !deref.can_set_property(key) {
                    return None
                }
                if !deref.can_get_property(key) {
                    next_object = generator();
                    deref.set(key, ShyValue::Object(next_object.shallow_clone()));
                }
                else { 
                    match deref.get(key) {
                        Some(ShyValue::Object(obj)) => next_object = obj.shallow_clone(),
                        _ => return None
                    }
                }
            }
            current_object = next_object;
        }
        Some(current_object)
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

    #[test]
    fn vivify() {
        let mut shy_obj = ShyObject::share(Rc::new(RefCell::new(HashMap::new())));
        let shy_value = ShyValue::Object(shy_obj.shallow_clone());
        let expected_qty = 3;

        match shy_obj.vivify(vec!["customers", "smith", "orders"], || ShyObject::empty()) {
            Some(customers_smith_orders) => {
                {
                    let mut deref = customers_smith_orders.as_deref_mut();
                    deref.set("quantity", expected_qty.into());
                }
                match shy_value.get("customers").get("smith").get("orders").get("quantity") {
                    ShyValue::Scalar(ShyScalar::Integer(actual_qty)) => { 
                        assert_eq!(expected_qty, actual_qty);
                        ()
                    },
                    _ => assert!(false)
                }
            },
            None => assert!(false)
        }
    }

}
