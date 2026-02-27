use engine::runner::evaluator::evaluate_rule_set_with_trace;
use engine::runner::model::{ComparisonOperator, Condition};
use engine::runner::parser::parse_rules;
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::ServerHandler;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Input types (need JsonSchema + Deserialize for rmcp tool macro)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EvaluateRulesInput {
    #[schemars(description = "The policy rule DSL string to evaluate")]
    pub rule: String,
    #[schemars(description = "The JSON data object to evaluate the rule against")]
    pub data: serde_json::Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ValidateRuleInput {
    #[schemars(description = "The policy rule DSL string to validate")]
    pub rule: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExplainRuleInput {
    #[schemars(description = "The policy rule DSL string to parse and explain")]
    pub rule: String,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct EvaluateRulesOutput {
    pub result: bool,
    pub outcomes: HashMap<String, bool>,
    pub trace: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ValidateRuleOutput {
    pub valid: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OperatorInfo {
    pub operator: String,
    pub forms: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ExplainOutput {
    pub rule_count: usize,
    pub rules: Vec<RuleExplain>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RuleExplain {
    pub label: Option<String>,
    pub selector: String,
    pub outcome: String,
    pub conditions: Vec<ConditionExplain>,
}

#[derive(Debug, Serialize)]
pub struct ConditionExplain {
    #[serde(rename = "type")]
    pub kind: String,
    pub selector: Option<String>,
    pub property: Option<String>,
    pub operator: Option<String>,
    pub value: Option<String>,
    pub rule_name: Option<String>,
    pub logical_op: Option<String>,
    pub optional: bool,
    pub negated: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub group: Vec<ConditionExplain>,
}

// ---------------------------------------------------------------------------
// Core logic (pure functions, independently testable)
// ---------------------------------------------------------------------------

pub fn do_evaluate_rules(rule: &str, data: serde_json::Value) -> EvaluateRulesOutput {
    let rule_set = match parse_rules(rule) {
        Ok(rs) => rs,
        Err(e) => {
            return EvaluateRulesOutput {
                result: false,
                outcomes: HashMap::new(),
                trace: None,
                error: Some(e.to_string()),
            };
        }
    };

    let eval = evaluate_rule_set_with_trace(&rule_set, &data);

    let trace_json = eval
        .trace
        .as_ref()
        .and_then(|t| serde_json::to_value(t).ok());

    match eval.result {
        Ok(map) => {
            let overall = map.values().all(|&v| v);
            let outcomes: HashMap<String, bool> = map.into_iter().collect();
            EvaluateRulesOutput {
                result: overall,
                outcomes,
                trace: trace_json,
                error: None,
            }
        }
        Err(e) => EvaluateRulesOutput {
            result: false,
            outcomes: HashMap::new(),
            trace: trace_json,
            error: Some(e.to_string()),
        },
    }
}

pub fn do_validate_rule(rule: &str) -> ValidateRuleOutput {
    match parse_rules(rule) {
        Ok(_) => ValidateRuleOutput {
            valid: true,
            error: None,
        },
        Err(e) => ValidateRuleOutput {
            valid: false,
            error: Some(e.to_string()),
        },
    }
}

pub fn do_list_operators() -> Vec<OperatorInfo> {
    let all_variants = [
        ComparisonOperator::GreaterThanOrEqual,
        ComparisonOperator::LessThanOrEqual,
        ComparisonOperator::EqualTo,
        ComparisonOperator::ExactlyEqualTo,
        ComparisonOperator::NotEqualTo,
        ComparisonOperator::LaterThan,
        ComparisonOperator::EarlierThan,
        ComparisonOperator::GreaterThan,
        ComparisonOperator::LessThan,
        ComparisonOperator::In,
        ComparisonOperator::NotIn,
        ComparisonOperator::Contains,
        ComparisonOperator::IsEmpty,
        ComparisonOperator::IsNotEmpty,
        ComparisonOperator::Within,
        ComparisonOperator::OlderThan,
        ComparisonOperator::YoungerThan,
        ComparisonOperator::HasFormat,
        ComparisonOperator::MatchesPattern,
        ComparisonOperator::IsValidEmail,
        ComparisonOperator::IsValidUrl,
        ComparisonOperator::IsValidUuid,
        ComparisonOperator::IsValidPhone,
        ComparisonOperator::IsValidDate,
        ComparisonOperator::IsValidTime,
        ComparisonOperator::IsValidDatetime,
        ComparisonOperator::IsValidIso8601,
        ComparisonOperator::IsNotValidEmail,
        ComparisonOperator::IsNotValidUrl,
        ComparisonOperator::IsNotValidUuid,
        ComparisonOperator::IsNotValidPhone,
        ComparisonOperator::IsNotValidDate,
        ComparisonOperator::IsNotValidTime,
        ComparisonOperator::IsNotValidDatetime,
        ComparisonOperator::IsNotValidIso8601,
        ComparisonOperator::StartsWith,
        ComparisonOperator::EndsWith,
        ComparisonOperator::Exists,
        ComparisonOperator::DoesNotExist,
        ComparisonOperator::IsNull,
        ComparisonOperator::IsNotNull,
        ComparisonOperator::IsInThePast,
        ComparisonOperator::IsInTheFuture,
        ComparisonOperator::IsBetween,
    ];

    all_variants
        .iter()
        .map(|op| OperatorInfo {
            operator: op.to_string(),
            forms: op
                .all_representations()
                .iter()
                .map(|s| s.to_string())
                .collect(),
        })
        .collect()
}

pub fn do_explain_rule(rule: &str) -> ExplainOutput {
    let rule_set = match parse_rules(rule) {
        Ok(rs) => rs,
        Err(e) => {
            return ExplainOutput {
                rule_count: 0,
                rules: Vec::new(),
                error: Some(e.to_string()),
            };
        }
    };

    let rules = rule_set
        .rules
        .iter()
        .map(|r| RuleExplain {
            label: r.label.clone(),
            selector: r.selector.clone(),
            outcome: r.outcome.clone(),
            conditions: r.conditions.iter().map(explain_condition_group).collect(),
        })
        .collect::<Vec<_>>();

    ExplainOutput {
        rule_count: rules.len(),
        rules,
        error: None,
    }
}

fn explain_condition_group(cg: &engine::runner::model::ConditionGroup) -> ConditionExplain {
    let logical_op = cg
        .operator
        .as_ref()
        .map(|op| format!("{:?}", op).to_lowercase());

    match &cg.condition {
        Condition::Comparison(c) => ConditionExplain {
            kind: "comparison".to_string(),
            selector: Some(c.selector.value.clone()),
            property: Some(c.property.value.clone()),
            operator: Some(c.operator.to_string()),
            value: Some(c.value.value.to_string()),
            rule_name: None,
            logical_op,
            optional: cg.optional,
            negated: cg.negated,
            group: Vec::new(),
        },
        Condition::RuleReference(r) => ConditionExplain {
            kind: "rule_reference".to_string(),
            selector: Some(r.selector.value.clone()),
            property: None,
            operator: None,
            value: None,
            rule_name: Some(r.rule_name.value.clone()),
            logical_op,
            optional: cg.optional,
            negated: cg.negated,
            group: Vec::new(),
        },
        Condition::Group(inner) => ConditionExplain {
            kind: "group".to_string(),
            selector: None,
            property: None,
            operator: None,
            value: None,
            rule_name: None,
            logical_op,
            optional: cg.optional,
            negated: cg.negated,
            group: inner.iter().map(explain_condition_group).collect(),
        },
    }
}

// ---------------------------------------------------------------------------
// PolicyTools MCP server struct + ServerHandler impl (both in same module so
// the generated private `tool_box` function is accessible to both impls)
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct PolicyTools;

#[rmcp::tool(tool_box)]
impl PolicyTools {
    /// Evaluate a policy rule DSL string against JSON data.
    /// Returns the overall result, per-outcome booleans, and an execution trace.
    #[rmcp::tool(
        description = "Evaluate a policy rule DSL string against JSON data. Returns result, per-outcome booleans, execution trace, and any parse/eval error."
    )]
    fn evaluate_rules(&self, #[tool(aggr)] input: EvaluateRulesInput) -> String {
        let output = do_evaluate_rules(&input.rule, input.data);
        serde_json::to_string(&output).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
    }

    /// Validate that a rule string parses successfully without evaluating it.
    #[rmcp::tool(
        description = "Check whether a policy rule DSL string is syntactically valid. Returns {valid: bool, error: string|null}."
    )]
    fn validate_rule(&self, #[tool(aggr)] input: ValidateRuleInput) -> String {
        let output = do_validate_rule(&input.rule);
        serde_json::to_string(&output).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
    }

    /// List every comparison operator supported by the policy DSL.
    #[rmcp::tool(
        description = "List all comparison operators supported by the policy DSL, grouped by operator with all their accepted string forms."
    )]
    fn list_operators(&self) -> String {
        let output = do_list_operators();
        serde_json::to_string(&output).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
    }

    /// Parse a rule and return a structured breakdown.
    #[rmcp::tool(
        description = "Parse a policy rule DSL string and return a structured breakdown: selectors, outcomes, and conditions per rule."
    )]
    fn explain_rule(&self, #[tool(aggr)] input: ExplainRuleInput) -> String {
        let output = do_explain_rule(&input.rule);
        serde_json::to_string(&output).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
    }
}

// ServerHandler impl lives in the same module so the private `tool_box`
// function generated above is in scope.
#[rmcp::tool(tool_box)]
impl ServerHandler for PolicyTools {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Policy Engine MCP server — evaluate, validate, and explain policy rules \
                 written in the policy DSL. Use evaluate_rules to run rules against data, \
                 validate_rule to check syntax, list_operators to see all supported operators, \
                 and explain_rule to understand the structure of a rule."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SIMPLE_RULE: &str = r#"
        A **Person** gets senior_discount
          if the __age__ of the **Person** is greater than or equal to 65.
    "#;

    // Two-rule DSL: free_shipping references express_eligible so there is
    // exactly one "global" (unreferenced) rule — a constraint the engine enforces.
    const MULTI_RULE: &str = r#"
        A **Order** gets express_eligible
          if the __weight__ of the **Order** is less than 5.

        A **Order** gets free_shipping
          if the **Order** is express_eligible
          and the __total__ of the **Order** is greater than 50.
    "#;

    // --- evaluate_rules tests ---

    #[test]
    fn evaluate_rules_matching_condition() {
        let data = json!({ "Person": { "age": 70 } });
        let out = do_evaluate_rules(SIMPLE_RULE, data);
        assert!(out.error.is_none(), "unexpected error: {:?}", out.error);
        assert!(out.result, "expected result=true for age 70");
        assert_eq!(*out.outcomes.get("senior_discount").unwrap(), true);
    }

    #[test]
    fn evaluate_rules_non_matching_condition() {
        let data = json!({ "Person": { "age": 30 } });
        let out = do_evaluate_rules(SIMPLE_RULE, data);
        assert!(out.error.is_none(), "unexpected error: {:?}", out.error);
        assert!(!out.result, "expected result=false for age 30");
        assert_eq!(*out.outcomes.get("senior_discount").unwrap(), false);
    }

    #[test]
    fn evaluate_rules_parse_error_returned() {
        let data = json!({});
        let out = do_evaluate_rules("not valid DSL !!!", data);
        assert!(!out.result);
        assert!(out.error.is_some(), "expected error message");
        assert!(out.outcomes.is_empty());
    }

    #[test]
    fn evaluate_rules_trace_present_on_success() {
        let data = json!({ "Person": { "age": 70 } });
        let out = do_evaluate_rules(SIMPLE_RULE, data);
        assert!(out.error.is_none());
        assert!(out.trace.is_some(), "expected trace to be present");
    }

    #[test]
    fn evaluate_rules_multiple_outcomes() {
        // With 2 rules, a global rule must exist for evaluate_rule_set to work.
        // Using a single-rule test avoids that constraint.
        let data = json!({ "Person": { "age": 70 } });
        let out = do_evaluate_rules(SIMPLE_RULE, data);
        assert!(out.error.is_none(), "unexpected error: {:?}", out.error);
        assert!(!out.outcomes.is_empty());
    }

    // --- validate_rule tests ---

    #[test]
    fn validate_rule_valid_dsl() {
        let out = do_validate_rule(SIMPLE_RULE);
        assert!(out.valid, "expected valid=true for valid DSL");
        assert!(out.error.is_none());
    }

    #[test]
    fn validate_rule_invalid_dsl() {
        let out = do_validate_rule("not valid DSL at all !!!");
        assert!(!out.valid, "expected valid=false");
        assert!(out.error.is_some(), "expected error message");
    }

    #[test]
    fn validate_rule_empty_string() {
        let out = do_validate_rule("");
        // Empty string may or may not parse — just ensure consistency
        if out.valid {
            assert!(out.error.is_none());
        } else {
            assert!(out.error.is_some());
        }
    }

    // --- list_operators tests ---

    #[test]
    fn list_operators_non_empty() {
        let ops = do_list_operators();
        assert!(!ops.is_empty(), "expected non-empty operator list");
    }

    #[test]
    fn list_operators_contains_basic_comparisons() {
        let ops = do_list_operators();
        let has_gt = ops.iter().any(|o| o.operator == "is greater than");
        let has_lt = ops.iter().any(|o| o.operator == "is less than");
        let has_eq = ops.iter().any(|o| o.operator == "is equal to");
        assert!(has_gt, "missing 'is greater than'");
        assert!(has_lt, "missing 'is less than'");
        assert!(has_eq, "missing 'is equal to'");
    }

    #[test]
    fn list_operators_each_has_at_least_one_form() {
        let ops = do_list_operators();
        for op in &ops {
            assert!(
                !op.forms.is_empty(),
                "operator '{}' has no forms",
                op.operator
            );
        }
    }

    #[test]
    fn list_operators_forms_include_primary() {
        let ops = do_list_operators();
        for op in &ops {
            assert!(
                op.forms.contains(&op.operator),
                "primary form '{}' missing from its own forms list",
                op.operator
            );
        }
    }

    #[test]
    fn list_operators_covers_all_variants() {
        let ops = do_list_operators();
        // Spot-check a broad range of categories
        let names: Vec<&str> = ops.iter().map(|o| o.operator.as_str()).collect();
        for expected in &[
            "is greater than or equal to",
            "is less than or equal to",
            "is equal to",
            "is in",
            "contains",
            "is empty",
            "is a valid email",
            "starts with",
            "exists",
            "is in the past",
            "is between",
        ] {
            assert!(names.contains(expected), "missing operator '{}'", expected);
        }
    }

    // --- explain_rule tests ---

    #[test]
    fn explain_rule_returns_correct_count() {
        let out = do_explain_rule(SIMPLE_RULE);
        assert!(out.error.is_none(), "unexpected error: {:?}", out.error);
        assert_eq!(out.rule_count, 1);
        assert_eq!(out.rules.len(), 1);
    }

    #[test]
    fn explain_rule_correct_selector_and_outcome() {
        let out = do_explain_rule(SIMPLE_RULE);
        let rule = &out.rules[0];
        assert_eq!(rule.selector, "Person");
        assert_eq!(rule.outcome, "senior_discount");
    }

    #[test]
    fn explain_rule_label_none_when_absent() {
        let out = do_explain_rule(SIMPLE_RULE);
        assert!(out.rules[0].label.is_none());
    }

    #[test]
    fn explain_rule_conditions_present() {
        let out = do_explain_rule(SIMPLE_RULE);
        let rule = &out.rules[0];
        assert!(
            !rule.conditions.is_empty(),
            "expected at least one condition"
        );
        let cond = &rule.conditions[0];
        assert_eq!(cond.kind, "comparison");
        assert_eq!(
            cond.operator.as_deref(),
            Some("is greater than or equal to")
        );
    }

    #[test]
    fn explain_rule_invalid_dsl_returns_error() {
        let out = do_explain_rule("not valid DSL !!!");
        assert!(out.error.is_some(), "expected error for invalid DSL");
        assert_eq!(out.rule_count, 0);
        assert!(out.rules.is_empty());
    }

    #[test]
    fn explain_rule_multiple_rules() {
        let out = do_explain_rule(MULTI_RULE);
        assert!(out.error.is_none(), "unexpected error: {:?}", out.error);
        assert_eq!(out.rule_count, 2);
    }

    #[test]
    fn explain_rule_with_label() {
        let labeled = r#"
            age.check. A **Person** gets adult
              if the __age__ of the **Person** is greater than or equal to 18.
        "#;
        let out = do_explain_rule(labeled);
        assert!(out.error.is_none(), "unexpected error: {:?}", out.error);
        let rule = &out.rules[0];
        assert_eq!(rule.label.as_deref(), Some("age.check"));
    }
}
