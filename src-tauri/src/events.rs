//! Typed payload of the application event bus.

use crate::commitgen::CommitGenEvent;
use crate::git_log::GitStepEvent;
use crate::scanner::ScanProgressEvent;

#[derive(Debug, Clone)]
pub enum AppEvent {
    FsChanged(String),
    GitStep(GitStepEvent),
    ScanProgress(ScanProgressEvent),
    CommitGen(CommitGenEvent),
}
