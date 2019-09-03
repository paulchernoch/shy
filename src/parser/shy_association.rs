use std::collections::HashMap;
use std::collections::hash_map::Keys;
use core::iter::Cloned;
use std::fmt::Debug;
use std::cell::RefCell;
use std::rc::Rc;
use super::shy_token::ShyValue;

// `Any` allows us to do dynamic typecasting.
use std::any::Any;

/// Permit the getting and setting of properties where the key is always a string slice and the value is a ShyValue. 
pub trait ShyAssociation {
    /// Set the named property to a new value and return the previous value.
    /// If it is not permitted to set this property or it had no value previously, return None.
    fn set(&mut self, property_name: &'static str, property_value: ShyValue) -> Option<ShyValue>;

    /// Get the value of the named property.
    /// If the property has no value or the property does not exist, return None.
    fn get(&self, property_name: &'static str) -> Option<&ShyValue>;

    /// True if is is possible to set the named property. 
    /// This may be true even if the property does not currently have a value.
    fn can_set_property(&self, property_name: &'static str) -> bool;

    /// True if the property currently has a value that can be retrieved, false otherwise.
    fn can_get_property(&self, property_name: &'static str) -> bool;

    fn keys<'a>(&'a self) -> Box<dyn Iterator<Item=&'static str> + 'a>;

    /// An &Any can be cast to a reference to a concrete type.
    fn as_any(&self) -> &Any;

    /// Compare two ShyAssociation for equality.
    fn equals_association(&self, other: &ShyAssociation) -> bool;

    /// Create a deep copy of the ShyAssociation and box it up in an Rc and RefCell, to preserve interior mutability.
    fn clone_association(&self) -> Rc<RefCell<ShyAssociation>>;
}

impl ShyAssociation for HashMap<&'static str, ShyValue> {
    fn set(&mut self, property_name: &'static str, property_value: ShyValue) -> Option<ShyValue> {
        self.insert(property_name, property_value)
    }

    fn get(&self, property_name: &'static str) -> Option<&ShyValue> {
        HashMap::get(self, property_name)
    }

    fn can_set_property(&self, _property_name: &'static str) -> bool {
        true
    }

    fn can_get_property(&self, property_name: &'static str) -> bool {
        self.contains_key(property_name)
    }

    fn keys<'a>(&'a self) -> Box<dyn Iterator<Item=&'static str> + 'a> {
        Box::new(self.keys().cloned())
    }

    fn as_any(&self) -> &Any {
        self
    }

    fn equals_association(&self, other: &ShyAssociation) -> bool {
        // Do a type-safe casting. If the types are different,
        // return false, otherwise test the values for equality.
        other
            .as_any()
            .downcast_ref::<Self>()
            .map_or(false, |a| self == a)
    }

    fn clone_association(&self) -> Rc<RefCell<ShyAssociation>> {
        Rc::new(RefCell::new(self.clone()))
    }
}


#[cfg(test)]
/// Tests of ShyAssociation trait methods as implemented for HashMap.
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[allow(unused_imports)]
    use spectral::prelude::*;

    #[test]
    fn can_get_property() {
        let mut dictionary : HashMap<&'static str, ShyValue> = HashMap::new();
        let value: ShyValue = "Webster".into();
        dictionary.set("name", value);
        let association: &mut ShyAssociation = &mut dictionary;
        asserting("Can get property when Key is defined").that(&association.can_get_property("name")).is_equal_to(true);
        asserting("Can not get property when Key is not defined").that(&association.can_get_property("age")).is_equal_to(false);
    }

    #[test]
    fn can_set_property() {
        let mut dictionary : HashMap<&'static str, ShyValue> = HashMap::new();
        let value: ShyValue = "Webster".into();
        dictionary.set("name", value);
        let association: &mut ShyAssociation = &mut dictionary;
        asserting("Can set property when Key is defined").that(&association.can_set_property("name")).is_equal_to(true);
        asserting("Can set property when Key is not defined").that(&association.can_set_property("age")).is_equal_to(true);
    }

    #[test]
    fn get() {
        let mut dictionary : HashMap<&'static str, ShyValue> = HashMap::new();
        let value: ShyValue = "Webster".into();
        dictionary.set("name", value.clone());
        let association: &dyn ShyAssociation = &dictionary;
        asserting("get returns value when Key is defined").that(&association.get("name").unwrap()).is_equal_to(&value);
        asserting("get returns None when Key is not defined").that(&association.get("age")).is_equal_to(None);
    }

    #[test]
    fn set() {
        let mut dictionary : HashMap<&str, ShyValue> = HashMap::new();
        let value: ShyValue = "Webster".into();
        let new_value: ShyValue = "Merriam".into();
        let age: ShyValue = 50.into();
        dictionary.set("name", value.clone());
        let association: &mut dyn ShyAssociation = &mut dictionary;
        asserting("set returns old value when Key is defined").that(&association.set("name", new_value.clone()).unwrap()).is_equal_to(&value);
        asserting("set returns None when Key is not defined").that(&association.set("age", age.clone())).is_equal_to(&None);
        asserting("get returns different value after set").that(&association.get("name").unwrap()).is_equal_to(&new_value);
    }

    #[test]
    fn equality() {
        let mut dictionary1 : HashMap<&str, ShyValue> = HashMap::new();
        let value: ShyValue = "Webster".into();
        let new_value: ShyValue = "Merriam".into();
        let age: ShyValue = 50.into();
        dictionary1.set("name", value.clone());
        let mut dictionary2 = dictionary1.clone();
        let association1: &mut ShyAssociation = &mut dictionary1;
        let association2: &mut ShyAssociation = &mut dictionary2;
        // DISABLE UNTIL WE GET EQUALITY WORKING
        //        asserting("equality works for ShyAssociations").that(*association1 == *association2).is_equal_to(true);
        
    }
}
