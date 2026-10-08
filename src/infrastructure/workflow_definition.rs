use color_eyre::{Result, eyre::eyre};
use serde_yaml_ng::Value;

use crate::domain::{InputKind, WorkflowInput};

/// Reads `on.workflow_dispatch.inputs` from the YAML of a workflow file.
/// A workflow without manual inputs (or without `workflow_dispatch`) yields
/// an empty list.
pub fn parse_dispatch_inputs(yaml: &str) -> Result<Vec<WorkflowInput>> {
    let doc: Value = serde_yaml_ng::from_str(yaml)?;
    let Some(mapping) = doc.as_mapping() else {
        return Err(eyre!("workflow file is not a YAML mapping"));
    };

    // some YAML parsers read the bare key `on` as the boolean `true`
    let on = mapping.get("on").or_else(|| mapping.get(Value::Bool(true)));
    let Some(Value::Mapping(triggers)) = on else {
        return Ok(Vec::new());
    };
    let Some(Value::Mapping(dispatch)) = triggers.get("workflow_dispatch") else {
        return Ok(Vec::new());
    };
    let Some(Value::Mapping(inputs)) = dispatch.get("inputs") else {
        return Ok(Vec::new());
    };

    Ok(inputs
        .iter()
        .filter_map(|(name, spec)| Some(parse_input(name.as_str()?, spec)))
        .collect())
}

fn parse_input(name: &str, spec: &Value) -> WorkflowInput {
    let field = |key: &str| spec.get(key);
    let kind = match field("type").and_then(Value::as_str) {
        Some("boolean") => InputKind::Boolean,
        Some("number") => InputKind::Number,
        Some("choice") => InputKind::Choice(
            field("options")
                .and_then(Value::as_sequence)
                .map(|options| options.iter().filter_map(scalar_to_string).collect())
                .unwrap_or_default(),
        ),
        // `string`, `environment` and anything unknown are plain text
        _ => InputKind::Text,
    };

    WorkflowInput {
        name: name.to_string(),
        description: field("description")
            .and_then(Value::as_str)
            .map(str::to_string),
        required: field("required").and_then(Value::as_bool).unwrap_or(false),
        default: field("default").and_then(scalar_to_string),
        kind,
    }
}

fn scalar_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_input_types_in_order() {
        let yaml = r#"
name: Deploy
on:
  push:
  workflow_dispatch:
    inputs:
      environment:
        description: Where to deploy
        type: choice
        options: [dev, staging, prod]
        default: staging
        required: true
      dry_run:
        type: boolean
        default: true
      replicas:
        type: number
        default: 3
      version:
        description: Tag to build
"#;
        let inputs = parse_dispatch_inputs(yaml).unwrap();
        let names: Vec<&str> = inputs.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, ["environment", "dry_run", "replicas", "version"]);

        assert_eq!(
            inputs[0],
            WorkflowInput {
                name: "environment".into(),
                description: Some("Where to deploy".into()),
                required: true,
                default: Some("staging".into()),
                kind: InputKind::Choice(vec!["dev".into(), "staging".into(), "prod".into()]),
            }
        );
        assert_eq!(inputs[1].kind, InputKind::Boolean);
        assert_eq!(inputs[1].default.as_deref(), Some("true"));
        assert_eq!(inputs[2].kind, InputKind::Number);
        assert_eq!(inputs[2].default.as_deref(), Some("3"));
        assert_eq!(inputs[3].kind, InputKind::Text);
        assert!(!inputs[3].required);
    }

    #[test]
    fn no_inputs_for_other_trigger_forms() {
        for yaml in [
            "on: push",
            "on: [push, workflow_dispatch]",
            "on:\n  workflow_dispatch:\n",
            "on:\n  workflow_dispatch: {}\n",
            "on:\n  pull_request:\n",
            "name: no triggers",
        ] {
            assert!(parse_dispatch_inputs(yaml).unwrap().is_empty(), "{yaml}");
        }
    }

    #[test]
    fn invalid_yaml_is_an_error() {
        assert!(parse_dispatch_inputs("on: [unclosed").is_err());
    }

    #[test]
    fn choice_without_options_is_empty() {
        let yaml = "on:\n  workflow_dispatch:\n    inputs:\n      x:\n        type: choice\n";
        let inputs = parse_dispatch_inputs(yaml).unwrap();
        assert_eq!(inputs[0].kind, InputKind::Choice(Vec::new()));
    }
}
