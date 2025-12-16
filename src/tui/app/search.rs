use regex::Regex;

use super::state::RebuildJob;

/// Direction to use when choosing the next match to focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchDirection {
    Forward,
    Backward,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchHit {
    pub line: usize,
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug)]
pub struct SearchState {
    pub query: String,
    pub direction: SearchDirection,
    pub editing: bool,
    pub regex: Option<Regex>,
    pub matches: Vec<SearchHit>,
    pub line_lookup: Vec<Vec<usize>>,
    pub active: Option<usize>,
    pub error: Option<String>,
}

impl SearchState {
    #[must_use]
    pub fn new(direction: SearchDirection) -> Self {
        Self {
            query: String::new(),
            direction,
            editing: true,
            regex: None,
            matches: Vec::new(),
            line_lookup: Vec::new(),
            active: None,
            error: None,
        }
    }

    #[must_use]
    pub fn has_query(&self) -> bool {
        !self.query.is_empty()
    }

    pub fn clear_results(&mut self) {
        self.regex = None;
        self.matches.clear();
        self.line_lookup.clear();
        self.active = None;
    }

    pub fn push_char(&mut self, ch: char) {
        self.query.push(ch);
    }

    pub fn pop_char(&mut self) {
        self.query.pop();
    }

    pub fn set_direction(&mut self, direction: SearchDirection) {
        self.direction = direction;
    }

    #[must_use]
    pub fn matches_for_line(&self, line: usize) -> Option<&[usize]> {
        self.line_lookup.get(line).map(Vec::as_slice)
    }

    #[must_use]
    pub fn active_hit(&self) -> Option<&SearchHit> {
        self.active.and_then(|idx| self.matches.get(idx))
    }

    pub fn advance_next(&mut self) {
        if self.matches.is_empty() {
            self.active = None;
            return;
        }
        let next = match self.active {
            Some(idx) => (idx + 1) % self.matches.len(),
            None => 0,
        };
        self.active = Some(next);
    }

    pub fn advance_prev(&mut self) {
        if self.matches.is_empty() {
            self.active = None;
            return;
        }
        let prev = match self.active {
            Some(0) | None => self.matches.len().saturating_sub(1),
            Some(idx) => idx - 1,
        };
        self.active = Some(prev);
    }

    pub fn recompute_matches(&mut self, job: &RebuildJob, baseline_line: usize) {
        let previous_selection = self.active.and_then(|idx| self.matches.get(idx).cloned());
        self.matches.clear();
        self.line_lookup = Vec::new();
        self.active = None;

        if self.query.is_empty() {
            self.regex = None;
            self.error = None;
            return;
        }

        let regex = match Regex::new(&self.query) {
            Ok(r) => r,
            Err(err) => {
                self.regex = None;
                self.error = Some(err.to_string());
                return;
            }
        };
        self.regex = Some(regex.clone());
        self.error = None;

        let total_lines = job.output.len();
        if total_lines == 0 {
            self.line_lookup = Vec::new();
            self.active = None;
            return;
        }

        self.line_lookup = vec![Vec::new(); total_lines];
        for (line_idx, entry) in job.output.iter().enumerate() {
            let normalized = normalize_for_search(&entry.text);
            for mat in regex.find_iter(&normalized) {
                let hit = SearchHit {
                    line: line_idx,
                    start: mat.start(),
                    end: mat.end(),
                };
                let hit_idx = self.matches.len();
                self.matches.push(hit);
                if let Some(bucket) = self.line_lookup.get_mut(line_idx) {
                    bucket.push(hit_idx);
                }
            }
        }

        if self.matches.is_empty() {
            self.active = None;
            return;
        }

        if let Some(prev) = previous_selection
            && let Some(idx) = self
                .matches
                .iter()
                .position(|hit| hit.line == prev.line && hit.start == prev.start)
            {
                self.active = Some(idx);
                return;
            }

        self.active = match self.direction {
            SearchDirection::Forward => self
                .matches
                .iter()
                .enumerate()
                .find(|(_, hit)| hit.line >= baseline_line)
                .map(|(idx, _)| idx)
                .or(Some(0)),
            SearchDirection::Backward => self
                .matches
                .iter()
                .enumerate()
                .rev()
                .find(|(_, hit)| hit.line <= baseline_line)
                .map(|(idx, _)| idx)
                .or_else(|| Some(self.matches.len().saturating_sub(1))),
        };
    }
}

fn normalize_for_search(text: &str) -> String {
    let segment = text.rsplit('\r').next().unwrap_or(text);
    segment.replace('\t', "    ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::app::OutputStream;
    use crate::tui::app::state::{RebuildJob, RebuildOutputLine};
    use std::collections::VecDeque;

    fn job_with_lines(lines: &[&str]) -> RebuildJob {
        let mut output = VecDeque::new();
        for line in lines {
            output.push_back(RebuildOutputLine {
                stream: OutputStream::Stdout,
                text: line.to_string(),
            });
        }
        RebuildJob {
            image: "img".into(),
            container: None,
            entry_path: std::path::PathBuf::from("."),
            source_dir: std::path::PathBuf::from("."),
            status: crate::tui::app::RebuildStatus::Running,
            output,
            error: None,
        }
    }

    #[test]
    fn recompute_matches_tracks_lines_and_active_forward() {
        let job = job_with_lines(&["alpha", "beta", "alphabet"]);
        let mut search = SearchState::new(SearchDirection::Forward);
        search.query = "alpha".into();
        search.editing = false;
        search.recompute_matches(&job, 0);

        assert_eq!(search.matches.len(), 2);
        assert_eq!(search.matches[0].line, 0);
        assert_eq!(search.matches[1].line, 2);
        assert_eq!(search.active, Some(0));
        assert!(search.error.is_none());
    }

    #[test]
    fn recompute_matches_tracks_backward_direction() {
        let job = job_with_lines(&["a", "b", "a"]);
        let mut search = SearchState::new(SearchDirection::Backward);
        search.query = "a".into();
        search.editing = false;
        search.recompute_matches(&job, 1);

        assert_eq!(search.matches.len(), 2);
        assert_eq!(search.active, Some(0));
        assert_eq!(search.matches[0].line, 0);
    }

    #[test]
    fn advance_wraps_over_matches() {
        let job = job_with_lines(&["foo", "foo"]);
        let mut search = SearchState::new(SearchDirection::Forward);
        search.query = "foo".into();
        search.editing = false;
        search.recompute_matches(&job, 0);

        search.advance_next();
        assert_eq!(search.active, Some(1));
        search.advance_next();
        assert_eq!(search.active, Some(0));

        search.advance_prev();
        assert_eq!(search.active, Some(1));
    }
}
