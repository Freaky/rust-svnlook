use std::process::Command;
use std::path::PathBuf;

use crate::{SvnError, SvnlookCommand};

#[derive(Debug)]
pub struct SvnDiffBuilder {
    repository: PathBuf,
    command: Command,
}

impl SvnDiffBuilder {
    pub(crate) fn new(repository: &PathBuf, mut command: Command) -> Self {
        command.arg("diff");

        Self {
            repository: repository.clone(),
            command
        }
    }

    pub fn no_diff_deleted(&mut self) -> &mut Self {
        self.command.arg("--no-diff-deleted");
        self
    }

    pub fn no_diff_added(&mut self) -> &mut Self {
        self.command.arg("--no-diff-added");
        self
    }

    pub fn diff_copy_from(&mut self) -> &mut Self {
        self.command.arg("--diff-copy-from");
        self
    }

    pub fn ignore_properties(&mut self) -> &mut Self {
        self.command.arg("--ignore-properties");
        self
    }

    pub fn properties_only(&mut self) -> &mut Self {
        self.command.arg("--properties-only");
        self
    }

    pub fn ignore_whitespace_change(&mut self) -> &mut Self {
        self.command.args(&["-x", "-b"]);
        self
    }

    pub fn ignore_all_whitespace(&mut self) -> &mut Self {
        self.command.args(&["-x", "-w"]);
        self
    }

    pub fn ignore_eol_style(&mut self) -> &mut Self {
        self.command.args(&["-x", "--ignore-eof-style"]);
        self
    }

    pub fn show_c_function_name(&mut self) -> &mut Self {
        self.command.args(&["-x", "-p"]);
        self
    }

    pub fn show_c_function_names(&mut self) -> &mut Self {
        self.command.args(&["-x", "-p"]);
        self
    }

    pub fn context_lines(&mut self, lines: u32) -> &mut Self {
        self.command.arg("-x");
        self.command.arg(format!("-U{}", lines));
        self
    }

    pub fn revision(&mut self, revision: u64) -> &mut Self {
        self.command.arg(format!("-r{}", revision));
        self
    }

    pub fn spawn(&mut self) -> Result<SvnlookCommand, SvnError> {
        self.command.arg("--");
        self.command.arg(&self.repository);
        SvnlookCommand::spawn(&mut self.command)
    }
}
