use std::collections::HashSet;
use serde::{Serialize, Deserialize};
use super::expression::Expressive;

#[derive(Serialize, Deserialize, Debug)]
pub enum ReferenceType {
    Definition,
    Dependency,
    ExternalDependency,
    Unknown
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Variables (or property chains) referenced by an expression, either as definitions or dependencies on other expressions (or the context).
pub struct References {
    /// Variables that are defined in an expression or set of expressions
    pub definitions : Vec<String>,

    /// Variables that are either external dependencies not defined in any expression, or are internal dependencies 
    /// that are used in one expression but defined in another. Until all expressions are processed,
    /// it is impossible to distinguish the two cases. 
    pub dependencies : Vec<String>,

    /// Variables that are not defined in any expressions and are referenced by at least one expression. 
    pub external_dependencies : Vec<String>
}

impl References {

    pub fn new() -> Self {
        References {
            definitions : Vec::new(),
            dependencies : Vec::new(),
            external_dependencies : Vec::new()
        }
    }

    pub fn get_reference_type(&self, variable : &String) -> ReferenceType {
        if self.definitions.contains(variable) { ReferenceType::Definition }
        else if self.dependencies.contains(variable) { ReferenceType::Dependency }
        else if self.external_dependencies.contains(variable) { ReferenceType::ExternalDependency }
        else { ReferenceType::Unknown }
    }

    /// Determine the cumulative effect of following a series of expressions that have collectively defined
    /// the variables in self by an expression that defines or depends on the expressions in additional_references.
    /// 
    /// 1. If additional_references has dependencies that are neither defined in self nor external_dependencies in self,
    /// return None, because it means the expression will attempt to retrieve an uninitialized variable. 
    /// 
    /// 2. If additional_references has definitions that are marked as dependencies or external_dependencies in self,
    /// return None, because that means that expressions were processed out of order. Expressions with a definition
    /// are supposed to be processed before all expressions that depend on it. A circular reference can cause this problem. 
    /// 
    /// 3. Otherwise, append the values in additional_references to the appropriate slots in a clone of self,
    /// taking care to not duplicate any names. 
    /// 
    /// Note: It is permissible for two expressions to define the same variable. It is likely that they have non-overlapping 
    /// applicability tests that would cause the variable to only be defined once. If the applicability tests overlap,
    /// then the variable may be defined twice, meaning that the order in which they are executed may matter.
    pub fn follow_by(&self, additional_references : &Self) -> Option<References> {
        let mut result = self.clone();
        for variable in &additional_references.dependencies {
            match self.get_reference_type(&variable) {
                ReferenceType::Definition => (),
                ReferenceType::Dependency => return None,
                ReferenceType::ExternalDependency => (),
                ReferenceType::Unknown => return None
            }
        }
        for variable in &additional_references.definitions {
            match self.get_reference_type(&variable) {
                ReferenceType::Definition => (), // Already defined, but that is okay. 
                ReferenceType::Dependency => return None, // Incorrect ordering of expressions!
                ReferenceType::ExternalDependency => return None, // Inconsistent!
                ReferenceType::Unknown => { result.definitions.push(variable.clone()) }
            }
        }
        for variable in &additional_references.external_dependencies {
            match self.get_reference_type(&variable) {
                ReferenceType::Definition => return None, // Inconsistent! 
                ReferenceType::Dependency => return None, // Incorrect ordering of expressions!
                ReferenceType::ExternalDependency => (), // Already known, but that is okay
                ReferenceType::Unknown => { result.external_dependencies.push(variable.clone()) }
            }
        }
        Some(result)
    }

    /// True if there are no internal or external dependencies.
    pub fn has_no_dependencies(&self) -> bool {
        self.dependencies.len() == 0 && self.external_dependencies.len() == 0
    }

    /// True if there are no internal dependencies.
    pub fn has_no_internal_dependencies(&self) -> bool {
        self.dependencies.len() == 0
    }

    /// True if there is an internal dependency on the given variable.
    pub fn has_internal_dependency_on(&self, variable: &String) -> bool {
        self.dependencies.contains(variable)
    }

    /// True if there is an external dependency on the given variable.
    pub fn has_external_dependency_on(&self, variable: &String) -> bool {
        self.external_dependencies.contains(variable)
    }

    /// Given a complete set of Expressions, infer which variable references are external dependencies.
    /// 
    ///   - If a variable has at least one Expression that defines it, assume it is not an external dependency. 
    ///   - If a variable only occurs as a dependency and never as a definition, assume that it is an external dependency. 
    /// 
    /// External dependencies are assumed to be provided by the ExecutionContext. 
    pub fn infer_external_dependencies<'a, X>(expressions : &mut Vec<X>) -> HashSet<String> 
    where X : Expressive<'a> + Sized
    {
        let mut all_definitions = HashSet::new();
        let mut all_dependencies = HashSet::new();
        for used_rro in expressions.iter().map(|expr| expr.express().variables_used()) {
            let used_opt = used_rro.read().unwrap();
            {
                let used = used_opt.as_ref().unwrap();
                all_definitions.extend(used.definitions.iter().cloned());
                all_dependencies.extend(used.dependencies.iter().cloned());
            }
            // Assume that we start out not having any known external dependencies. 
        }
        // Assume that all dependencies that never receive definitions are external_dependencies. 
        // They must be provided by the context. 
        let diffs: HashSet<String> = all_dependencies.difference(&all_definitions).map(|diff| diff.clone()).collect();
        diffs
    }

    /// Now that we know the external dependencies, move all variables from self.dependencies to self.external_dependencies
    /// that are found in externals.
    pub fn apply_external_dependencies(&mut self, externals : &HashSet<String>) {
        for dependency in externals {
            if let Some(position) = self.external_dependencies.iter().position(|s| *s == *dependency) {
                self.external_dependencies.push(self.dependencies.swap_remove(position));
            }
        }
    }
    
}
