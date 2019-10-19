
// use std::rc::Rc;
// use std::cell::RefCell;
use std::sync::{Arc,RwLock};
use std::fmt::*;
use std::ops::{Deref, DerefMut};
use std::collections::HashMap;

use super::shy_association::*;
use super::shy_token::ShyValue;

/// A Holder for a ShyAssociation that permits the implementation of Clone, PartialEq, and Debug for a Trait object.
pub struct ShyObject {
    pub association: Arc<RwLock<dyn ShyAssociation + Send + Sync>>
}

impl ShyObject {
    /// Create a new ShyObject that contains a clone of the given ShyAssociation.
    pub fn new<A : ShyAssociation + Send + Sync>(value: &A) -> ShyObject {
        ShyObject {
            association: value.clone_association()
        }
    }

    /// Create a new ShyObject that contains a clone of the given Arc, ensuring that the underlying ShyAssociation is NOT cloned.
    pub fn share(wrapped: Arc<RwLock<dyn ShyAssociation + Send + Sync>>) -> ShyObject {
        ShyObject {
            association: wrapped.clone()
        }
    }

    pub fn empty() -> ShyObject {
        ShyObject {
            association: Arc::new(RwLock::new(HashMap::new()))
        }
    }

    /// Create a shallow clone of this ShyObject that has a new Arc that points to the same RwLock and hence the same ShyAssociation.
    pub fn shallow_clone(&self) -> ShyObject {
        ShyObject {
            association: self.association.clone()
        }
    }

    pub fn as_deref(&self) -> impl Deref<Target = dyn ShyAssociation + Send + Sync> {
        self.association.read().unwrap()
    }

    pub fn as_deref_mut(&self) -> impl DerefMut<Target = dyn ShyAssociation + Send + Sync> {
        self.association.write().unwrap()
    }

    /// Follow a path from the given ShyObject down to one of its descendants and retrieve that descendant as an Option.
    /// If any levels of the hierarchy are missing and can be added, add them using the supplied generator.
    /// If at any stage it is not permitted to add or follow a given property in tht path, return None.
    /// If the path vector is empty, return a shallow clone of self.
    ///    self ....... Object to look inside
    ///    path ....... Names of properties to get, in order, to descend from self to its children.
    ///    depth ...... Number of path components to follow. 
    ///    generator .. If a level of object is missing, use this to construct it. 
    pub fn vivify<'a, S>(&'a mut self, path: Vec<S>, depth: usize, generator: impl Fn() -> ShyObject) -> Option<ShyObject>
    where S : Display {
        if path.len() == 0 || depth == 0 {
            return Some(self.shallow_clone())
        }
        let mut current_object = self.shallow_clone();
        let mut next_object;
        for key in path[0..depth].iter().map(|k| k.to_string()) {
            {
                let mut deref = current_object.as_deref_mut();
                if !deref.can_set_property(&key) {
                    return None
                }
                if !deref.can_get_property(&key) {
                    next_object = generator();
                    deref.set(&key, ShyValue::Object(next_object.shallow_clone()));
                }
                else { 
                    match deref.get(&key) {
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
            association: self.association.read().unwrap().clone_association()
        }
    }
}

impl PartialEq for ShyObject {
    fn eq(&self, other: &ShyObject) -> bool {
        let other_assoc: &dyn ShyAssociation = &*other.association.read().unwrap();
        self.association.read().unwrap().equals_association(other_assoc)
    }
}

impl Debug for ShyObject {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.association.read().unwrap().to_indented_string(2, 2))
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[allow(unused_imports)]
    use spectral::prelude::*;

    use super::super::shy_scalar::ShyScalar;

    #[test]
    fn as_deref() {
        let mut dictionary1 : HashMap<String, ShyValue> = HashMap::new();
        set_into(&mut dictionary1, "name", "The Doctor");
        set_into(&mut dictionary1, "season", 12);
        set_into(&mut dictionary1, "popular", true);
        let wrapped: Arc<RwLock<dyn ShyAssociation + Send + Sync>> = Arc::new(RwLock::new(dictionary1));
        let shy_obj = ShyObject::share(wrapped);
        let deref = shy_obj.as_deref();
        let actual = deref.get("name");
        let expected = ShyValue::Scalar(ShyScalar::String("The Doctor".to_owned()));
        assert_eq!(Option::Some(&expected), actual);
    }

    #[test]
    fn vivify() {
        let mut shy_obj = ShyObject::share(Arc::new(RwLock::new(HashMap::new())));
        let shy_value = ShyValue::Object(shy_obj.shallow_clone());
        let expected_qty = 3;

        match shy_obj.vivify(vec!["customers", "smith", "orders"], 3, || ShyObject::empty()) {
            Some(customers_smith_orders) => {
                {
                    let mut deref = customers_smith_orders.as_deref_mut();
                    deref.set("quantity", expected_qty.into());
                }
                match shy_value.get_safe("customers").get_safe("smith").get_safe("orders").get_safe("quantity") {
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
