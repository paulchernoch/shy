use serde::{Serialize, Deserialize};
use crate::parser::expression::Expression;
use crate::parser::expression::Expressive;

use crate::parser::execution_context::ExecutionContext;
use crate::parser::shy_token::ShyValue;
use crate::parser::shy_scalar::ShyScalar;

pub mod ruleset;


#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
/// Indicates the type of a Rule.
pub enum RuleType {
    /// Indicates that the rule defines one or more properties. 
    /// It may return a boolean or a value of any supported type. 
    Property,

    /// Indicates that the Rule returns a boolean and is a pass/fail rule. 
    Predicate
}


/// A Rule for use in a rule engine. 
/// 
/// A Rule is an expression that conforms to a special format. 
/// The values of most properties of the rule can be extracted from the Expression.
/// For example, if the expression has the phrase `rule.description = "Is the well in the US?"`
/// then the description property can be set. These property chains can be used
/// to specify Rule properties:
/// 
///   - rule.name
///   - rule.id
///   - rule.description
///   - rule.type
///   - rule.category
///   - rule.sequence
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Rule<'a> {
    /// Name of rule.
    /// 
    /// If not supplied in the expression via an assignment to property chain `rule.name`, the name will be "Rule" + id. 
    pub name : String,

    /// Unique integer id for a rule. 
    /// 
    /// If the expression sets `rule.id`, that value overrides the value passed into the Rule constructor.
    pub id : usize,

    /// User friendly description of the rule's intent. 
    /// 
    /// If the expression does not assign a string to property chain `rule.description`, 
    /// the description will be `Option::None`.
    /// 
    /// If the rule is a predicate, this should be phrased as a question such that if 
    /// the answer to the question is "Yes", the rule passes.
    /// 
    /// ## Examples: 
    ///    - Is the mud depth less than 5000 ft?   GOOD! It is specific and a true answer implies passing. 
    ///    - Is the mud depth too great?            BAD! It is not specific and a true answer implies failing. 
    pub description : Option<String>,

    /// Type of rule, which defaults to Predicate. 
    pub rule_type : RuleType,

    /// Optional rule category. 
    pub category : Option<String>,

    /// Required expression to be evaluated for this rule.
    ///
    /// The expression should return a boolean for Predicate type rules, but may return any value for Property type rules. 
    expression : Expression<'a>
}

impl<'a> Expressive<'a> for Rule<'a> {
    fn express(&self) -> &Expression<'a> { &self.expression }
    fn express_mut(&mut self) -> &mut Expression<'a> { &mut self.expression }
}

impl<'a, 'b : 'a> Rule<'a> {
    /// Construct a Rule, deriving some of its properties from variables in the context if they are present. 
    /// To accomplish this, the expression is evaluated. Even if the context lacks information necessary
    /// to completely evaluate the Rule, a well defined expression will be able to set the properties
    /// that begin with "Rule." as part of their names.
    pub fn new<S>(expression_source : S, id : usize, ctx_opt : Option<ExecutionContext<'b>>) -> Rule<'a>
    where S : Into<String> {
        let expr_str = expression_source.into();
        let mut context = match ctx_opt {
            Some(ctx) => ctx,
            None => ExecutionContext::<'a>::default()
        };
        let expression_to_use = Expression::new(expr_str);
        let _ = expression_to_use.exec(&mut context);

        // Attempt to get Rule property values from the context, but use defaults if not found.
        let id_to_use = match context.load_str_chain("rule.id") {
            Some(ShyValue::Scalar(ShyScalar::Integer(rule_id))) => rule_id as usize,
            _ => id
        };
        let name_to_use = match context.load_str_chain("rule.name") {
            Some(ShyValue::Scalar(ShyScalar::String(rule_name))) => rule_name,
            _ => format!("Rule{}", id)
        };
        let description_to_use = match context.load_str_chain("rule.description") {
            Some(ShyValue::Scalar(ShyScalar::String(rule_description))) => Some(rule_description),
            _ => None
        };
        let rule_type_to_use = match context.load_str_chain("rule.type") {
            Some(ShyValue::Scalar(ShyScalar::String(ref rule_type))) if rule_type == "Property" => RuleType::Property,
            _ => RuleType::Predicate
        };
        let category_to_use = match context.load_str_chain("rule.category") {
            Some(ShyValue::Scalar(ShyScalar::String(rule_category))) => Some(rule_category),
            _ => None
        };
        Rule {
            name : name_to_use,
            id : id_to_use,
            description : description_to_use,
            rule_type : rule_type_to_use,
            category : category_to_use,
            expression : expression_to_use
        }
    }

    /// Names of all properties and property chains that this rule defines. 
    /// 
    /// This list is derived by analyzing the Expression, to find all variables and property chains
    /// that are defined within the expression before their first use.
    ///
    /// Property chains that begin with "rule." are excluded, as they define meta data common to many rules. 
    pub fn definitions(&self) -> Vec<String> {
        self.expression.get_references().definitions.iter().filter(|s| ! (*s).starts_with("rule.")).map(|s| s.to_string()).collect()
    }

    /// Names of all properties and property chains that this rule references but does not also define.
    /// 
    /// This list is derived by analyzing the Expression, to find all variables and property chains
    /// that are used within the expression either without being defined or before they are defined.
    ///
    /// Property chains that begin with "rule." are excluded, as they define meta data common to many rules. 
    pub fn dependencies(&self) -> Vec<String> {
        self.expression.get_references().dependencies.iter().filter(|s| ! (*s).starts_with("rule.")).map(|s| s.to_string()).collect()
    }
}

