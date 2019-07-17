use std::collections::HashMap;
use super::shy_token::ShyValue;

/// Permit the getting and setting of properties where the key is always a string slice and the value is a ShyValue. 
pub trait ShyAssociation<'a> {
    /// Set the named property to a new value and return the previous value.
    /// If it is not permitted to set this property or it had no value previously, return None.
    fn set(&mut self, property_name: &'a str, property_value: ShyValue) -> Option<ShyValue>;

    /// Get the value of the named property.
    /// If the property has no value or the property does not exist, return None.
    fn get(&self, property_name: &'a str) -> Option<&ShyValue>;

    /// True if is is possible to set the named property. 
    /// This may be true even if the property does not currently have a value.
    fn can_set_property(&self, property_name: &'a str) -> bool;

    /// True if the property currently has a value that can be retrieved, false otherwise.
    fn can_get_property(&self, property_name: &'a str) -> bool;
}

impl<'a> ShyAssociation<'a> for HashMap<&'a str, ShyValue> {
    fn set(&mut self, property_name: &'a str, property_value: ShyValue) -> Option<ShyValue> {
        self.insert(property_name, property_value.clone())
    }

    fn get(&self, property_name: &'a str) -> Option<&ShyValue> {
        HashMap::get(self, property_name)
    }

    fn can_set_property(&self, _property_name: &'a str) -> bool {
        true
    }

    fn can_get_property(&self, property_name: &'a str) -> bool {
        self.contains_key(property_name)
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
        let mut dictionary : HashMap<&str, ShyValue> = HashMap::new();
        let value: ShyValue = "Webster".into();
        dictionary.set("name", value);
        let association: &ShyAssociation = &dictionary;
        asserting("Can get property when Key is defined").that(&association.can_get_property("name")).is_equal_to(true);
        asserting("Can not get property when Key is not defined").that(&association.can_get_property("age")).is_equal_to(false);
    }

    #[test]
    fn can_set_property() {
        let mut dictionary : HashMap<&str, ShyValue> = HashMap::new();
        let value: ShyValue = "Webster".into();
        dictionary.set("name", value);
        let association: &ShyAssociation = &dictionary;
        asserting("Can set property when Key is defined").that(&association.can_set_property("name")).is_equal_to(true);
        asserting("Can set property when Key is not defined").that(&association.can_set_property("age")).is_equal_to(true);
    }

    #[test]
    fn get() {
        let mut dictionary : HashMap<&str, ShyValue> = HashMap::new();
        let value: ShyValue = "Webster".into();
        dictionary.set("name", value.clone());
        let association: &ShyAssociation = &dictionary;
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
        let association: &mut ShyAssociation = &mut dictionary;
        asserting("set returns old value when Key is defined").that(&association.set("name", new_value.clone()).unwrap()).is_equal_to(&value);
        asserting("set returns None when Key is not defined").that(&association.set("age", age.clone())).is_equal_to(&None);
        asserting("get returns different value after set").that(&association.get("name").unwrap()).is_equal_to(&new_value);
    }
}
