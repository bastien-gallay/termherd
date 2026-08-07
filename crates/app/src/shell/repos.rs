//! Declaring a repo into the sidebar (`F-repo-add`). The native picker and the
//! window file-drop both land here, so the normalisation that keeps a
//! declaration and a discovery of one repository on a single sidebar key has
//! exactly one caller — and one place to test.

use std::path::Path;

use iced::Task;

use super::{Message, Shell};

impl Shell {
    /// Normalise a picked folder and declare it. `None` is a cancelled dialog.
    ///
    /// The work is a handful of `stat`s, so it runs here rather than in a
    /// `Task`: nothing is gained by deferring it, and a future is where the
    /// runtime traps live.
    pub(super) fn declare_repo(&mut self, picked: Option<&Path>) -> Task<Message> {
        let Some(picked) = picked else {
            return Task::none();
        };
        let Some(path) = termherd_scan::normalize_repo_path(picked) else {
            // A refusal is a dead end for the user, so it is said out loud
            // rather than swallowed (Q5): a vanished drop is indistinguishable
            // from a broken button.
            tracing::warn!(
                picked = %picked.display(),
                "cannot add to the sidebar: the path does not exist"
            );
            return Task::none();
        };
        tracing::info!(repo = %path.display(), "repo declared in the sidebar");
        let effects = self.core.apply(termherd_core::Event::DeclareRepo(
            path.display().to_string(),
        ));
        self.perform(effects)
    }
}
