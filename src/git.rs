/*
 * Copyright (c) 2025 Hangzhou Guanwaii Technology Co,.Ltd.
 *
 * This source code is licensed under the MIT License,
 * which is located in the LICENSE file in the source tree's root directory.
 *
 * File: git.rs
 * Author: mingcheng (mingcheng@apache.org)
 * File Created: 2025-03-01 21:55:54
 *
 * Modified By: mingcheng (mingcheng@apache.org)
 * Last Modified: 2025-07-11 18:36:58
 */

use git2::{Repository, RepositoryOpenFlags, Signature, StatusOptions};
use log::trace;
use std::error::Error;
use std::path::Path;

pub struct Git {
    repository: Repository,
}

impl Git {
    pub fn new(path: &str) -> Result<Git, Box<dyn Error>> {
        trace!("opening repository at {path}");
        let repository = Repository::open_ext(path, RepositoryOpenFlags::empty(), vec![path])?;

        trace!("repository opened successfully");
        if let Some(work_dir) = repository.workdir() {
            trace!("the repository workdir is: {work_dir:?}");
        } else {
            return Err("the repository has no workdir".into());
        }

        Ok(Git { repository })
    }

    /// Commit the changes in the repository
    pub fn commit(&self, message: &str, need_signoff: bool) -> Result<(), Box<dyn Error>> {
        // Get the current index (staged changes)

        let mut message = message.to_string();
        let mut index = self.repository.index()?;

        // Write the index to the repository
        let oid = index.write_tree()?;
        let tree = self.repository.find_tree(oid)?;

        // Get the HEAD commit
        let head = self.repository.head()?.peel_to_commit()?;

        // Create a new commit
        let author_name = self.get_author_name()?;
        let author_email = self.get_author_email()?;

        // Create a signature for the author and committer
        let signature = Signature::now(&author_name, &author_email)?;

        // If the --signoff option is enabled, add signoff to the commit message
        if need_signoff {
            trace!("signoff option is enabled, will add signoff to the commit message");

            // Add signoff to the commit message
            let signoff = format!("\n\nSigned-off-by: {author_name} <{author_email}>");
            message.push_str(&signoff);
        }

        match self.repository.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &message,
            &tree,
            &[&head],
        ) {
            Ok(_) => {
                trace!("commit created successfully");
                Ok(())
            }
            Err(e) => {
                trace!("failed to create commit: {e}");
                Err(Box::new(e))
            }
        }
    }

    /// Get the author email and name from the repository configuration
    pub fn get_author_email(&self) -> Result<String, Box<dyn Error>> {
        // Get the configuration of the repository
        let config = self.repository.config()?;

        // Get the user email from the configuration
        match config.get_string("user.email") {
            Ok(email) => {
                trace!("get author email: {email} from config `user.email`");
                Ok(email)
            }

            Err(e) => {
                trace!("failed to get author email: {e}");
                Err(Box::new(e))
            }
        }
    }

    pub fn get_author_name(&self) -> Result<String, Box<dyn Error>> {
        // Get the configuration of the repository
        let config = self.repository.config()?;

        // Get the user name from the configuration
        match config.get_string("user.name") {
            Ok(name) => {
                trace!("get author name: {name} from config `user.name`");
                Ok(name)
            }

            Err(e) => {
                trace!("failed to get author name: {e}");
                Err(Box::new(e))
            }
        }
    }

    /// Get the diff of the current repository
    pub fn get_diff(&self) -> Result<Vec<String>, Box<dyn Error>> {
        // Get the current index (staged changes)
        let index = self.repository.index()?;

        // Get the HEAD commit
        let head = self.repository.head()?.peel_to_commit()?;

        // Create diff options
        let mut diffopts = git2::DiffOptions::new();
        diffopts
            .show_binary(false)
            .force_binary(false)
            .ignore_submodules(true)
            .minimal(true);

        // Create status options to filter out ignored files
        let mut statusopts = StatusOptions::new();
        statusopts.include_untracked(false).include_ignored(false);

        self.repository.statuses(Some(&mut statusopts))?;

        // Get the diff between the HEAD and the index
        let diff = self.repository.diff_tree_to_index(
            Some(&head.tree()?),
            Some(&index),
            Some(&mut diffopts),
        )?;

        // Collect diff stats into strings
        // Iterate over the diff and print the changes, excluding ignored files
        let mut result = vec![];
        diff.print(git2::DiffFormat::Patch, |delta, _hunk, line| {
            if delta.new_file().path().is_some_and(|path| {
                path == Path::new("go.mod")
                    || path == Path::new("go.sum")
                    || path == Path::new("Cargo.lock")
            }) {
                return true; // Skip this file
            }

            result.push(String::from_utf8_lossy(line.content()).trim().to_string());
            true
        })?;

        Ok(result)
    }

    /// Get the latest `size` commit messages from the repository
    pub fn get_logs(&self, size: usize) -> Result<Vec<String>, Box<dyn Error>> {
        // Get the `size` latest commits starting from HEAD
        let mut revwalk = self.repository.revwalk()?;

        // Start from HEAD
        revwalk.push_head()?;

        // Set the sorting order
        revwalk.set_sorting(git2::Sort::TIME)?;

        // Collect the 5 latest commits
        let commits = revwalk
            .take(size)
            .filter_map(Result::ok)
            .filter_map(|oid| self.repository.find_commit(oid).ok())
            .map(|commit| commit.message().unwrap_or("").trim().to_string())
            .collect();

        Ok(commits)
    }
}

#[cfg(test)]
mod tests {
    use log::error;

    use super::*;

    fn setup() -> Result<Git, Box<dyn Error>> {
        let repo_path = std::env::var("TEST_REPO_PATH")
            .map_err(|_| "TEST_REPO_PATH environment variable not set")?;
        if repo_path.is_empty() {
            return Err("please specify the repository path".into());
        }
        Git::new(&repo_path)
    }

    #[test]
    fn test_new() {
        if setup().is_err() {
            error!("please specify the repository path");
            return;
        }

        assert!(setup().is_ok());
    }

    #[test]
    fn test_get_author_email() {
        let repo = setup();
        if repo.is_err() {
            error!("please specify the repository path");
            return;
        }

        let email = repo.unwrap().get_author_email();
        assert!(email.is_ok());
        assert!(!email.unwrap().is_empty());
    }

    #[test]
    fn test_get_author_name() {
        let repo = setup();
        if repo.is_err() {
            error!("please specify the repository path");
            return;
        }

        let name = repo.unwrap().get_author_name();
        assert!(name.is_ok());
        assert!(!name.unwrap().is_empty());
    }

    #[test]
    fn test_logs() {
        let repo = setup();
        if repo.is_err() {
            error!("please specify the repository path");
            return;
        }

        let logs = repo.unwrap().get_logs(5);
        assert!(logs.is_ok());
        assert_eq!(logs.unwrap().len(), 5);
    }

    #[test]
    fn test_diff() {
        let repo = setup();
        if repo.is_err() {
            error!("please specify the repository path");
            return;
        }

        let diffs = repo.unwrap().get_diff();
        assert!(diffs.is_ok());

        let diff_content = diffs.unwrap();
        assert_ne!(diff_content.len(), 0);
    }
}
