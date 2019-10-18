use std::collections::HashMap;
// use std::cell::RefCell;
// use std::rc::Rc;
use std::sync::{Arc,RwLock};
use itertools::sorted;
use super::shy_token::ShyValue;
use super::indent::IndentDisplay;
use super::indent::write_debug;

// `Any` allows us to do dynamic typecasting.
use std::any::Any;

/// Permit the getting and setting of properties where the key is always a string slice and the value is a ShyValue. 
pub trait ShyAssociation {
    /// Set the named property to a new value and return the previous value.
    /// If it is not permitted to set this property or it had no value previously, return None.
    fn set(&mut self, property_name: &str, property_value: ShyValue) -> Option<ShyValue>;

    /// Get the value of the named property.
    /// If the property has no value or the property does not exist, return None.
    fn get(&self, property_name: &str) -> Option<&ShyValue>;

    /// True if is is possible to set the named property. 
    /// This may be true even if the property does not currently have a value.
    fn can_set_property(&self, property_name: &str) -> bool;

    /// True if the property currently has a value that can be retrieved, false otherwise.
    fn can_get_property(&self, property_name: &str) -> bool;

    /// Boxes up an iterator over all the property names for the association.
    fn keys<'a>(&'a self) -> Box<dyn Iterator<Item=String> + 'a>;

    /// An &Any can be cast to a reference to a concrete type.
    fn as_any(&self) -> &dyn Any;

    /// Compare two ShyAssociation for equality.
    fn equals_association(&self, other: &dyn ShyAssociation) -> bool;

    /// Create a deep copy of the ShyAssociation and box it up in an Ar and RwLock, to preserve interior mutability.
    fn clone_association(&self) -> Arc<RwLock<dyn ShyAssociation>>;

    /// Supports the writing of a Debug formatter
    fn to_indented_string<'a>(&self, indent_by: usize, tab_size: usize) -> String;
}

/// Permit setting the value for a key in a ShyAssociation using any value that can be converted into a ShyValue.
/// Note: I would prefer that this be a method of the ShyAssociation trait, but doing so yields this error:
///    method `set_into` has generic type parameters rustc(E0038).
pub fn set_into<A : ShyAssociation, V: Into<ShyValue> + Sized>(association: &mut A, property_name: &str, property_value: V) -> Option<ShyValue> {
    association.set(property_name, property_value.into())
}

impl ShyAssociation for HashMap<String, ShyValue> {
    fn set(&mut self, property_name: &str, property_value: ShyValue) -> Option<ShyValue> {
        self.insert(property_name.into(), property_value)
    }

    fn get(&self, property_name: &str) -> Option<&ShyValue> {
        HashMap::get(self, property_name)
    }

    fn can_set_property(&self, _property_name: &str) -> bool {
        true
    }

    fn can_get_property(&self, property_name: &str) -> bool {
        self.contains_key(property_name)
    }

    fn keys<'a>(&'a self) -> Box<dyn Iterator<Item=String> + 'a> {
        Box::new(self.keys().cloned())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn equals_association(&self, other: &dyn ShyAssociation) -> bool {
        // Do a type-safe casting. If the types are different,
        // return false, otherwise test the values for equality.
        other
            .as_any()
            .downcast_ref::<Self>()
            .map_or(false, |a| self == a)
    }

    fn clone_association(&self) -> Arc<RwLock<dyn ShyAssociation>> {
        Arc::new(RwLock::new(self.clone()))
    }

    /// Format the key-value pairs as a string, indenting the given number of spaces
    /// and incrementing the indentation for the values, should any of them also be associations.
    fn to_indented_string<'a>(&'a self, indent_by: usize, tab_size: usize) -> String {
        let mut indented = String::new();
        indented.push_str(&"{\n".indent_display(indent_by));
        for key_ptr in sorted(self.keys()) {
            let key = key_ptr.clone();
            match self.get(&key) {
                Some(value) => {
                    let s = vec![key.as_str(), ": ", &write_debug(value, "Error")].concat();
                    indented.push_str(&s.indent_display(indent_by + tab_size))
                },
                None => indented.push_str(&"?".indent_display(indent_by + tab_size))
            }
            indented.push_str("\n");
        }
        indented.push_str(&"}\n".indent_display(indent_by));
        indented
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
        let mut dictionary : HashMap<String, ShyValue> = HashMap::new();
        let value: ShyValue = "Webster".into();
        dictionary.set("name", value);
        let association: &mut dyn ShyAssociation = &mut dictionary;
        asserting("Can get property when Key is defined").that(&association.can_get_property("name")).is_equal_to(true);
        asserting("Can not get property when Key is not defined").that(&association.can_get_property("age")).is_equal_to(false);
    }

    #[test]
    fn can_set_property() {
        let mut dictionary : HashMap<String, ShyValue> = HashMap::new();
        let value: ShyValue = "Webster".into();
        dictionary.set("name", value);
        let association: &mut dyn ShyAssociation = &mut dictionary;
        asserting("Can set property when Key is defined").that(&association.can_set_property("name")).is_equal_to(true);
        asserting("Can set property when Key is not defined").that(&association.can_set_property("age")).is_equal_to(true);
    }

    #[test]
    fn get() {
        let mut dictionary : HashMap<String, ShyValue> = HashMap::new();
        let value: ShyValue = "Webster".into();
        dictionary.set("name", value.clone());
        let association: &dyn ShyAssociation = &dictionary;
        asserting("get returns value when Key is defined").that(&association.get("name").unwrap()).is_equal_to(&value);
        asserting("get returns None when Key is not defined").that(&association.get("age")).is_equal_to(None);
    }

    #[test]
    fn set() {
        let mut dictionary : HashMap<String, ShyValue> = HashMap::new();
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
        let mut dictionary1 : HashMap<String, ShyValue> = HashMap::new();
        let value: ShyValue = "Webster".into();
        let age: ShyValue = 50.into();
        dictionary1.set("name", value.clone());
        dictionary1.set("age", age);
        let mut dictionary2 = dictionary1.clone();
        let association1: &mut dyn ShyAssociation = &mut dictionary1;
        let association2: &mut dyn ShyAssociation = &mut dictionary2;
        // DISABLE UNTIL WE GET EQUALITY WORKING
        asserting("equality works for ShyAssociations").that(&association1.equals_association(association2)).is_equal_to(true);
        
    }

    #[test]
    fn to_indented_string() {
        let mut dictionary1 : HashMap<String, ShyValue> = HashMap::new();
        set_into(&mut dictionary1, "name", "The Doctor");
        set_into(&mut dictionary1, "season", 12);
        set_into(&mut dictionary1, "popular", true);
        let actual = dictionary1.to_indented_string(0,2);
        println!("actual:\n{}\n:actual\n", actual);
        let expected = r#"{
  name: Scalar(String("The Doctor"))
  popular: Scalar(Boolean(true))
  season: Scalar(Integer(12))
}
"#;
        println!("expected:\n{}\n:expected\n", expected);

        asserting("to_indented_string works for a dictionary").that(&(actual == *expected)).is_equal_to(true);
    }
}
