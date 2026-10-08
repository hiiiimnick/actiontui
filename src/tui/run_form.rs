use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::domain::{InputKind, WorkflowInput};

/// What the app has to do after a key press in the form.
#[derive(Debug, Eq, PartialEq)]
pub enum FormAction {
    None,
    Cancel,
    Submit,
    /// The branch was edited; the workflow's inputs may differ on it.
    BranchChanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub label: String,
    pub description: Option<String>,
    pub required: bool,
    pub kind: InputKind,
    pub value: String,
}

impl Field {
    fn branch(branch: String) -> Self {
        Self {
            label: "branch".into(),
            description: Some("Branch or tag to run the workflow on".into()),
            required: true,
            kind: InputKind::Text,
            value: branch,
        }
    }

    fn from_input(input: WorkflowInput) -> Self {
        let value = match &input.kind {
            InputKind::Boolean => (input.default.as_deref() == Some("true")).to_string(),
            InputKind::Choice(options) => input
                .default
                .clone()
                .filter(|d| options.contains(d))
                .or_else(|| options.first().cloned())
                .unwrap_or_default(),
            InputKind::Text | InputKind::Number => input.default.clone().unwrap_or_default(),
        };
        Self {
            label: input.name,
            description: input.description,
            required: input.required,
            kind: input.kind,
            value,
        }
    }

    fn is_editable(&self) -> bool {
        matches!(self.kind, InputKind::Text | InputKind::Number)
    }

    /// Flips a boolean or moves a choice by `delta`, wrapping around.
    fn cycle(&mut self, delta: isize) {
        match &self.kind {
            InputKind::Boolean => self.value = (self.value != "true").to_string(),
            InputKind::Choice(options) if !options.is_empty() => {
                let current = options.iter().position(|o| *o == self.value).unwrap_or(0);
                let next = (current as isize + delta).rem_euclid(options.len() as isize);
                self.value = options[next as usize].clone();
            }
            _ => {}
        }
    }

    fn validate(&self) -> Result<(), String> {
        let empty = self.value.trim().is_empty();
        if self.required && empty {
            return Err(format!("'{}' is required", self.label));
        }
        if self.kind == InputKind::Number && !empty && self.value.trim().parse::<f64>().is_err() {
            return Err(format!("'{}' must be a number", self.label));
        }
        Ok(())
    }
}

/// The form to start a new run: the branch plus the inputs of the workflow.
///
/// Vim style: in normal mode `j`/`k` move between fields, `i` edits a text
/// field, `h`/`l` (or `Space`) change a boolean or choice, `Enter` runs and
/// `Esc` cancels. While editing, `Esc` or `Enter` returns to normal mode.
#[derive(Debug)]
pub struct RunForm {
    pub workflow_id: u64,
    pub workflow_name: String,
    /// The first field is always the branch.
    pub fields: Vec<Field>,
    pub selected: usize,
    pub editing: bool,
    /// Validation errors or problems loading the inputs.
    pub notice: Option<String>,
    branch_before_edit: String,
}

impl RunForm {
    pub fn new(
        workflow_id: u64,
        workflow_name: String,
        branch: String,
        inputs: Vec<WorkflowInput>,
        notice: Option<String>,
    ) -> Self {
        let mut form = Self {
            workflow_id,
            workflow_name,
            fields: vec![Field::branch(branch)],
            selected: 0,
            editing: false,
            notice,
            branch_before_edit: String::new(),
        };
        form.fields
            .extend(inputs.into_iter().map(Field::from_input));
        form
    }

    pub fn branch(&self) -> &str {
        self.fields[0].value.trim()
    }

    /// Replaces the workflow inputs, e.g. after switching the branch.
    pub fn set_inputs(&mut self, inputs: Vec<WorkflowInput>, notice: Option<String>) {
        self.fields.truncate(1);
        self.fields
            .extend(inputs.into_iter().map(Field::from_input));
        self.selected = 0;
        self.notice = notice;
    }

    /// The branch and the inputs to dispatch. Empty optional inputs are left
    /// out so the workflow's own defaults apply.
    pub fn values(&self) -> (String, HashMap<String, String>) {
        let inputs = self.fields[1..]
            .iter()
            .filter(|f| !f.value.trim().is_empty())
            .map(|f| (f.label.clone(), f.value.trim().to_string()))
            .collect();
        (self.branch().to_string(), inputs)
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> FormAction {
        if self.editing {
            self.handle_key_insert(key)
        } else {
            self.handle_key_normal(key)
        }
    }

    fn handle_key_normal(&mut self, key: KeyEvent) -> FormAction {
        let last = self.fields.len() - 1;
        match key.code {
            KeyCode::Char('j') => self.selected = (self.selected + 1).min(last),
            KeyCode::Char('k') => self.selected = self.selected.saturating_sub(1),
            KeyCode::Char('g') => self.selected = 0,
            KeyCode::Char('G') => self.selected = last,
            KeyCode::Char('i') | KeyCode::Char('a') => {
                if self.fields[self.selected].is_editable() {
                    self.editing = true;
                    self.branch_before_edit = self.fields[0].value.clone();
                }
            }
            // like vim's `D`: delete to the end of the line
            KeyCode::Char('D') => {
                if self.fields[self.selected].is_editable() {
                    self.fields[self.selected].value.clear();
                    return self.after_edit();
                }
            }
            KeyCode::Char('h') => self.fields[self.selected].cycle(-1),
            KeyCode::Char('l') | KeyCode::Char(' ') | KeyCode::Char('x') => {
                self.fields[self.selected].cycle(1)
            }
            KeyCode::Enter => return self.submit(),
            KeyCode::Esc | KeyCode::Char('q') => return FormAction::Cancel,
            _ => {}
        }
        FormAction::None
    }

    fn handle_key_insert(&mut self, key: KeyEvent) -> FormAction {
        let value = &mut self.fields[self.selected].value;
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                self.editing = false;
                return self.after_edit();
            }
            KeyCode::Backspace => {
                value.pop();
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => value.clear(),
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => value.push(c),
            _ => {}
        }
        FormAction::None
    }

    fn after_edit(&mut self) -> FormAction {
        if self.selected == 0 && self.fields[0].value != self.branch_before_edit {
            self.branch_before_edit = self.fields[0].value.clone();
            FormAction::BranchChanged
        } else {
            FormAction::None
        }
    }

    fn submit(&mut self) -> FormAction {
        for (index, field) in self.fields.iter().enumerate() {
            if let Err(message) = field.validate() {
                self.selected = index;
                self.notice = Some(message);
                return FormAction::None;
            }
        }
        FormAction::Submit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(form: &mut RunForm, keys: &str) -> FormAction {
        let mut action = FormAction::None;
        for c in keys.chars() {
            action = form.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        action
    }

    fn key(form: &mut RunForm, code: KeyCode) -> FormAction {
        form.handle_key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn input(name: &str, kind: InputKind, default: Option<&str>, required: bool) -> WorkflowInput {
        WorkflowInput {
            name: name.into(),
            description: None,
            required,
            default: default.map(Into::into),
            kind,
        }
    }

    fn form(inputs: Vec<WorkflowInput>) -> RunForm {
        RunForm::new(1, "Deploy".into(), "main".into(), inputs, None)
    }

    fn env_input() -> WorkflowInput {
        input(
            "env",
            InputKind::Choice(vec!["dev".into(), "staging".into(), "prod".into()]),
            Some("staging"),
            true,
        )
    }

    #[test]
    fn branch_is_the_first_field() {
        let form = form(vec![env_input()]);
        assert_eq!(form.fields[0].label, "branch");
        assert_eq!(form.branch(), "main");
        assert_eq!(form.fields.len(), 2);
    }

    #[test]
    fn defaults_are_applied() {
        let form = form(vec![
            env_input(),
            input("dry", InputKind::Boolean, Some("true"), false),
            input("flag", InputKind::Boolean, None, false),
            input("n", InputKind::Number, Some("3"), false),
            input(
                "bad",
                InputKind::Choice(vec!["a".into(), "b".into()]),
                Some("zzz"),
                false,
            ),
        ]);
        let values: Vec<&str> = form.fields.iter().map(|f| f.value.as_str()).collect();
        assert_eq!(values, ["main", "staging", "true", "false", "3", "a"]);
    }

    #[test]
    fn j_k_g_big_g_move_and_clamp() {
        let mut form = form(vec![env_input(), input("v", InputKind::Text, None, false)]);
        press(&mut form, "j");
        assert_eq!(form.selected, 1);
        press(&mut form, "jjj");
        assert_eq!(form.selected, 2);
        press(&mut form, "k");
        assert_eq!(form.selected, 1);
        press(&mut form, "g");
        assert_eq!(form.selected, 0);
        press(&mut form, "G");
        assert_eq!(form.selected, 2);
        press(&mut form, "kkkkk");
        assert_eq!(form.selected, 0);
    }

    #[test]
    fn h_and_l_cycle_choices_and_wrap() {
        let mut form = form(vec![env_input()]);
        press(&mut form, "j");
        press(&mut form, "l");
        assert_eq!(form.fields[1].value, "prod");
        press(&mut form, "l");
        assert_eq!(form.fields[1].value, "dev");
        press(&mut form, "h");
        assert_eq!(form.fields[1].value, "prod");
    }

    #[test]
    fn space_and_x_toggle_booleans() {
        let mut form = form(vec![input("dry", InputKind::Boolean, None, false)]);
        press(&mut form, "j ");
        assert_eq!(form.fields[1].value, "true");
        press(&mut form, "x");
        assert_eq!(form.fields[1].value, "false");
    }

    #[test]
    fn i_edits_text_and_esc_leaves_insert_mode() {
        let mut form = form(vec![input("v", InputKind::Text, None, false)]);
        press(&mut form, "ji");
        assert!(form.editing);
        // keys that are commands in normal mode are just text now
        press(&mut form, "jq1");
        assert_eq!(form.fields[1].value, "jq1");
        key(&mut form, KeyCode::Backspace);
        assert_eq!(form.fields[1].value, "jq");
        key(&mut form, KeyCode::Esc);
        assert!(!form.editing);
        // Esc in insert mode does not cancel the form
        assert_eq!(form.selected, 1);
    }

    #[test]
    fn i_does_nothing_on_boolean_and_choice() {
        let mut form = form(vec![
            env_input(),
            input("d", InputKind::Boolean, None, false),
        ]);
        press(&mut form, "ji");
        assert!(!form.editing);
        press(&mut form, "ji");
        assert!(!form.editing);
    }

    #[test]
    fn capital_d_clears_a_text_field() {
        let mut form = form(vec![input("v", InputKind::Text, Some("abc"), false)]);
        press(&mut form, "jD");
        assert_eq!(form.fields[1].value, "");
    }

    #[test]
    fn editing_the_branch_reports_a_change() {
        let mut form = form(vec![]);
        press(&mut form, "i");
        press(&mut form, "-x");
        assert_eq!(key(&mut form, KeyCode::Esc), FormAction::BranchChanged);
        assert_eq!(form.branch(), "main-x");

        // no change, no action
        press(&mut form, "i");
        assert_eq!(key(&mut form, KeyCode::Esc), FormAction::None);
    }

    #[test]
    fn q_and_esc_cancel_in_normal_mode() {
        assert_eq!(press(&mut form(vec![]), "q"), FormAction::Cancel);
        assert_eq!(key(&mut form(vec![]), KeyCode::Esc), FormAction::Cancel);
    }

    #[test]
    fn enter_submits_valid_input() {
        let mut form = form(vec![env_input()]);
        assert_eq!(key(&mut form, KeyCode::Enter), FormAction::Submit);
    }

    #[test]
    fn enter_rejects_missing_required_value_and_selects_it() {
        let mut form = form(vec![
            input("must", InputKind::Text, None, true),
            input("opt", InputKind::Text, None, false),
        ]);
        assert_eq!(key(&mut form, KeyCode::Enter), FormAction::None);
        assert_eq!(form.selected, 1);
        assert_eq!(form.notice.as_deref(), Some("'must' is required"));
    }

    #[test]
    fn enter_rejects_invalid_numbers() {
        let mut form = form(vec![input("n", InputKind::Number, Some("abc"), false)]);
        assert_eq!(key(&mut form, KeyCode::Enter), FormAction::None);
        assert_eq!(form.notice.as_deref(), Some("'n' must be a number"));
    }

    #[test]
    fn values_skip_empty_optional_inputs() {
        let mut form = form(vec![
            env_input(),
            input("empty", InputKind::Text, None, false),
            input("dry", InputKind::Boolean, None, false),
            input("v", InputKind::Text, Some(" 1.2 "), false),
        ]);
        press(&mut form, "g");
        let (branch, inputs) = form.values();
        assert_eq!(branch, "main");
        assert_eq!(inputs.len(), 3);
        assert_eq!(inputs["env"], "staging");
        assert_eq!(inputs["dry"], "false");
        assert_eq!(inputs["v"], "1.2");
    }

    #[test]
    fn set_inputs_keeps_the_branch() {
        let mut form = form(vec![env_input()]);
        press(&mut form, "ji");
        form.editing = false;
        form.set_inputs(
            vec![input("new", InputKind::Text, None, false)],
            Some("hi".into()),
        );
        assert_eq!(form.fields.len(), 2);
        assert_eq!(form.fields[1].label, "new");
        assert_eq!(form.branch(), "main");
        assert_eq!(form.selected, 0);
        assert_eq!(form.notice.as_deref(), Some("hi"));
    }
}
