//! Git actions exposed to the rest of the app.
//!
//! The `actions!` macro expands in this module, so the generated action types
//! (`ToggleStaged`, `StageAll`, ...) live at `crate::actions::*`. They are
//! re-exported from the crate root via `pub use actions::*;` so downstream
//! code can keep using `git::ToggleStaged`.

use gpui::{Action, actions};
use schemars::JsonSchema;
use serde::Deserialize;

actions!(
    git,
    [
        // per-hunk
        /// Toggles the staged state of the hunk or status entry at cursor.
        ToggleStaged,
        /// Stage status entries between an anchor entry and the cursor.
        StageRange,
        /// Stages the current hunk and moves to the next one.
        StageAndNext,
        /// Unstages the current hunk and moves to the next one.
        UnstageAndNext,
        /// Restores the selected hunks to their original state.
        #[action(deprecated_aliases = ["editor::RevertSelectedHunks"])]
        Restore,
        /// Restores the selected hunks to their original state and moves to the
        /// next one.
        RestoreAndNext,
        // per-file
        /// Shows git blame information for the current file.
        #[action(deprecated_aliases = ["editor::ToggleGitBlame"])]
        Blame,
        /// Shows the git history for the selected file, folder, or project.
        FileHistory,
        /// Opens a permalink for the selected file on its Git hosting provider.
        OpenFilePermalink,
        /// Copies a permalink for the selected file on its Git hosting provider.
        CopyFilePermalink,
        /// Opens the selected file in the editor without a diff view.
        ViewFile,
        /// Stages the current file.
        StageFile,
        /// Unstages the current file.
        UnstageFile,
        // per-section
        /// Stages every entry in the section containing the selected entry.
        StageSection,
        /// Unstages every entry in the section containing the selected entry.
        UnstageSection,
        // repo-wide
        /// Stages all changes in the repository.
        StageAll,
        /// Unstages all changes in the repository.
        UnstageAll,
        /// Stashes all changes in the repository, including untracked files.
        StashAll,
        /// Stashes tracked changes in the repository, leaving untracked files in place.
        StashTracked,
        /// Stashes staged changes in the repository, leaving unstaged changes in place.
        StashStaged,
        /// Pops the most recent stash.
        StashPop,
        /// Apply the most recent stash.
        StashApply,
        /// Restores all tracked files to their last committed state.
        RestoreTrackedFiles,
        /// Moves all untracked files to trash.
        TrashUntrackedFiles,
        /// Undoes the last commit, keeping changes in the working directory.
        Uncommit,
        /// Pushes commits to the remote repository.
        Push,
        /// Pushes commits to a specific remote branch.
        PushTo,
        /// Force pushes commits to the remote repository.
        ForcePush,
        /// Pulls changes from the remote repository.
        Pull,
        /// Pulls changes from the remote repository with rebase.
        PullRebase,
        /// Fetches changes from the remote repository.
        Fetch,
        /// Fetches changes from a specific remote.
        FetchFrom,
        /// Creates a new commit with staged changes.
        Commit,
        /// Runs the next commit with `git commit --no-verify`.
        SkipHooks,
        /// Amends the last commit with staged changes.
        Amend,
        /// Enable the --signoff option.
        Signoff,
        /// Cancels the current git operation.
        Cancel,
        /// Expands the commit message editor.
        ExpandCommitEditor,
        /// Toggles whether the commit message editor fills all the available
        /// vertical space within the git panel.
        ToggleFillCommitEditor,
        /// Generates a commit message using AI.
        GenerateCommitMessage,
        /// Initializes a new git repository.
        Init,
        /// Opens all modified files in the editor.
        OpenModifiedFiles,
        /// Opens the current file in a solo diff view.
        OpenFileDiff,
        /// Clones a repository.
        Clone,
        ViewCommit,
        /// Adds a file to .gitignore.
        AddToGitignore,
        /// Adds a file to the repository's .git/info/exclude.
        AddToGitInfoExclude,
        /// Copies the current branch name to the clipboard.
        CopyBranchName,
    ]
);

/// Renames a git branch.
#[derive(Clone, Debug, Default, PartialEq, Deserialize, JsonSchema, Action)]
#[action(namespace = git)]
#[serde(deny_unknown_fields)]
pub struct RenameBranch {
    /// The branch to rename.
    ///
    /// Default: the current branch.
    #[serde(default)]
    pub branch: Option<String>,
}

/// Restores a file to its last committed state, discarding local changes.
#[derive(Clone, Debug, Default, PartialEq, Deserialize, JsonSchema, Action)]
#[action(namespace = git, deprecated_aliases = ["editor::RevertFile"])]
#[serde(deny_unknown_fields)]
pub struct RestoreFile {
    #[serde(default)]
    pub skip_prompt: bool,
}
