use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::ListState;

use crate::domain::RepositorySummary;

/// A repository together with the profile (account) it was found with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoEntry {
    pub profile: String,
    pub summary: RepositorySummary,
}

impl RepoEntry {
    pub fn full_name(&self) -> String {
        format!(
            "{}/{}",
            self.summary.repository.owner, self.summary.repository.repo
        )
    }

    fn matches(&self, terms: &[String]) -> bool {
        let haystack = format!(
            "{} {} {}",
            self.full_name(),
            self.profile,
            self.summary.description.as_deref().unwrap_or("")
        )
        .to_lowercase();
        terms.iter().all(|term| haystack.contains(term))
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum PickerAction {
    None,
    Open(RepoEntry),
    Quit,
}

/// The searchable list of all repositories.
///
/// Vim style: `j`/`k` move, `g`/`G` jump, `/` (or `i`) searches, `Enter` opens
/// the selected repository and `Esc` clears the search. While searching every
/// key is text; `Enter` returns to the list and `Esc` clears the search.
/// The search matches when every word occurs in `owner/repo`, the profile or
/// the description, ignoring case.
#[derive(Debug)]
pub struct RepoPicker {
    entries: Vec<RepoEntry>,
    pub query: String,
    pub searching: bool,
    pub state: ListState,
    /// Profiles whose repositories could not be loaded, with the reason.
    pub notices: Vec<String>,
}

impl RepoPicker {
    pub fn new(entries: Vec<RepoEntry>, notices: Vec<String>) -> Self {
        let mut state = ListState::default();
        if !entries.is_empty() {
            state.select(Some(0));
        }
        Self {
            entries,
            query: String::new(),
            searching: false,
            state,
            notices,
        }
    }

    pub fn total(&self) -> usize {
        self.entries.len()
    }

    /// The entries matching the search.
    pub fn visible(&self) -> Vec<&RepoEntry> {
        let terms: Vec<String> = self
            .query
            .split_whitespace()
            .map(str::to_lowercase)
            .collect();
        self.entries.iter().filter(|e| e.matches(&terms)).collect()
    }

    pub fn selected(&self) -> Option<&RepoEntry> {
        let index = self.state.selected()?;
        self.visible().get(index).copied()
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> PickerAction {
        if self.searching {
            self.handle_key_search(key);
            PickerAction::None
        } else {
            self.handle_key_normal(key)
        }
    }

    fn handle_key_normal(&mut self, key: KeyEvent) -> PickerAction {
        let last = self.visible().len().saturating_sub(1);
        let current = self.state.selected().unwrap_or(0);
        match key.code {
            KeyCode::Char('j') => self.select(current + 1),
            KeyCode::Char('k') => self.select(current.saturating_sub(1)),
            KeyCode::Char('g') => self.select(0),
            KeyCode::Char('G') => self.select(last),
            KeyCode::Char('/') | KeyCode::Char('i') => self.searching = true,
            KeyCode::Esc => {
                self.query.clear();
                self.select(0);
            }
            KeyCode::Enter => {
                if let Some(entry) = self.selected() {
                    return PickerAction::Open(entry.clone());
                }
            }
            KeyCode::Char('q') => return PickerAction::Quit,
            _ => {}
        }
        PickerAction::None
    }

    fn handle_key_search(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => self.searching = false,
            KeyCode::Esc => {
                self.searching = false;
                self.query.clear();
                self.select(0);
            }
            KeyCode::Backspace => {
                if self.query.pop().is_none() {
                    self.searching = false;
                }
                self.select(0);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.query.clear();
                self.select(0);
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.query.push(c);
                self.select(0);
            }
            _ => {}
        }
    }

    /// Selects `index`, clamped to the visible entries.
    fn select(&mut self, index: usize) {
        let count = self.visible().len();
        self.state.select((count > 0).then(|| index.min(count - 1)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Repository;

    fn entry(profile: &str, owner: &str, repo: &str, description: Option<&str>) -> RepoEntry {
        RepoEntry {
            profile: profile.into(),
            summary: RepositorySummary {
                repository: Repository::new("github.com".into(), owner.into(), repo.into()),
                private: false,
                description: description.map(Into::into),
            },
        }
    }

    fn picker() -> RepoPicker {
        RepoPicker::new(
            vec![
                entry("personal", "me", "dotfiles", Some("My configuration")),
                entry(
                    "personal",
                    "me",
                    "actiontui",
                    Some("A TUI for GitHub Actions"),
                ),
                entry("work", "acme", "api-gateway", None),
                entry("work", "acme", "web-app", Some("Customer portal")),
            ],
            Vec::new(),
        )
    }

    fn press(picker: &mut RepoPicker, keys: &str) -> PickerAction {
        let mut action = PickerAction::None;
        for c in keys.chars() {
            action = picker.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        action
    }

    fn key(picker: &mut RepoPicker, code: KeyCode) -> PickerAction {
        picker.handle_key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn names(picker: &RepoPicker) -> Vec<String> {
        picker.visible().iter().map(|e| e.full_name()).collect()
    }

    #[test]
    fn starts_with_everything_and_the_first_selected() {
        let picker = picker();
        assert_eq!(picker.total(), 4);
        assert_eq!(picker.visible().len(), 4);
        assert_eq!(picker.selected().unwrap().full_name(), "me/dotfiles");
    }

    #[test]
    fn an_empty_picker_selects_nothing() {
        let mut picker = RepoPicker::new(Vec::new(), Vec::new());
        assert!(picker.selected().is_none());
        assert_eq!(key(&mut picker, KeyCode::Enter), PickerAction::None);
        press(&mut picker, "jkgG");
        assert!(picker.selected().is_none());
    }

    #[test]
    fn j_k_g_big_g_move_and_clamp() {
        let mut picker = picker();
        press(&mut picker, "jj");
        assert_eq!(picker.state.selected(), Some(2));
        press(&mut picker, "jjjj");
        assert_eq!(picker.state.selected(), Some(3));
        press(&mut picker, "k");
        assert_eq!(picker.state.selected(), Some(2));
        press(&mut picker, "g");
        assert_eq!(picker.state.selected(), Some(0));
        press(&mut picker, "G");
        assert_eq!(picker.state.selected(), Some(3));
        press(&mut picker, "kkkkkkk");
        assert_eq!(picker.state.selected(), Some(0));
    }

    #[test]
    fn slash_searches_and_filters_while_typing() {
        let mut picker = picker();
        press(&mut picker, "/");
        assert!(picker.searching);
        press(&mut picker, "act");
        assert_eq!(names(&picker), ["me/actiontui"]);
        // `j` and `q` are text while searching
        press(&mut picker, "jq");
        assert!(picker.visible().is_empty());
        assert!(picker.selected().is_none());
        key(&mut picker, KeyCode::Backspace);
        key(&mut picker, KeyCode::Backspace);
        assert_eq!(names(&picker), ["me/actiontui"]);
    }

    #[test]
    fn every_word_has_to_match_in_any_order_and_field() {
        let mut picker = picker();
        press(&mut picker, "/work API");
        assert_eq!(names(&picker), ["acme/api-gateway"]);

        picker.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        press(&mut picker, "portal acme");
        assert_eq!(names(&picker), ["acme/web-app"]);
    }

    #[test]
    fn search_ignores_case() {
        let mut picker = picker();
        press(&mut picker, "/ACTIONTUI");
        assert_eq!(names(&picker), ["me/actiontui"]);
    }

    #[test]
    fn enter_ends_the_search_and_keeps_the_filter() {
        let mut picker = picker();
        press(&mut picker, "/acme");
        key(&mut picker, KeyCode::Enter);
        assert!(!picker.searching);
        assert_eq!(picker.visible().len(), 2);
        // now `j` moves again
        press(&mut picker, "j");
        assert_eq!(picker.selected().unwrap().full_name(), "acme/web-app");
    }

    #[test]
    fn esc_clears_the_search_in_both_modes() {
        let mut picker = picker();
        press(&mut picker, "/acme");
        key(&mut picker, KeyCode::Esc);
        assert!(!picker.searching);
        assert_eq!(picker.visible().len(), 4);

        press(&mut picker, "/acme");
        key(&mut picker, KeyCode::Enter);
        key(&mut picker, KeyCode::Esc);
        assert_eq!(picker.visible().len(), 4);
        assert_eq!(picker.state.selected(), Some(0));
    }

    #[test]
    fn backspace_on_an_empty_query_leaves_the_search() {
        let mut picker = picker();
        press(&mut picker, "/");
        key(&mut picker, KeyCode::Backspace);
        assert!(!picker.searching);
    }

    #[test]
    fn ctrl_u_clears_the_query() {
        let mut picker = picker();
        press(&mut picker, "/acme");
        picker.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert_eq!(picker.query, "");
        assert!(picker.searching);
        assert_eq!(picker.visible().len(), 4);
    }

    #[test]
    fn enter_opens_the_selected_entry_of_the_filtered_list() {
        let mut picker = picker();
        press(&mut picker, "/work");
        key(&mut picker, KeyCode::Enter);
        press(&mut picker, "j");
        match key(&mut picker, KeyCode::Enter) {
            PickerAction::Open(entry) => {
                assert_eq!(entry.full_name(), "acme/web-app");
                assert_eq!(entry.profile, "work");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn q_quits_only_outside_the_search() {
        assert_eq!(press(&mut picker(), "q"), PickerAction::Quit);
        let mut searching = picker();
        press(&mut searching, "/");
        assert_eq!(press(&mut searching, "q"), PickerAction::None);
        assert_eq!(searching.query, "q");
    }
}
