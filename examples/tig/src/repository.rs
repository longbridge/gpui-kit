use std::{path::PathBuf, process::Command};

use gpui_kit::{SharedString, component::diff::DiffFile};

const HISTORY_LIMIT: usize = 200;

#[derive(Clone, Debug)]
pub(super) struct Commit {
    pub(super) hash: SharedString,
    pub(super) short_hash: SharedString,
    pub(super) author: SharedString,
    pub(super) date: SharedString,
    pub(super) subject: SharedString,
}

pub(super) struct CommitDetails {
    pub(super) message: SharedString,
    pub(super) files: Vec<DiffFile>,
}

#[derive(Clone)]
pub(super) struct Repository {
    path: PathBuf,
}

impl Repository {
    pub(super) fn new(path: PathBuf) -> Self {
        Self {
            path: std::fs::canonicalize(&path).unwrap_or(path),
        }
    }

    pub(super) fn label(&self) -> String {
        self.path.display().to_string()
    }

    pub(super) fn name(&self) -> String {
        self.path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Repository".into())
    }

    fn command(&self) -> Command {
        let mut command = Command::new("git");
        command.arg("--no-pager").arg("-C").arg(&self.path);
        command
    }

    fn read(&self, args: &[&str]) -> Result<String, String> {
        let output = self
            .command()
            .args(args)
            .output()
            .map_err(|error| format!("Couldn't run Git: {error}"))?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
        }
        String::from_utf8(output.stdout).map_err(|_| {
            "This Git output isn't UTF-8. Convert the source encoding to view it.".into()
        })
    }

    pub(super) fn history(&self) -> Result<Vec<Commit>, String> {
        // Verify the repository separately so an unborn HEAD is an empty state,
        // while a missing directory or a non-repository remains a useful error.
        self.read(&["rev-parse", "--git-dir"])?;
        let head = self
            .command()
            .args(["rev-parse", "--verify", "--quiet", "HEAD"])
            .output()
            .map_err(|error| format!("Couldn't read HEAD: {error}"))?;
        if !head.status.success() {
            return Ok(Vec::new());
        }
        let output = self.read(&[
            "log",
            "--no-show-signature",
            "--encoding=UTF-8",
            "--date=short",
            "-z",
            &format!("--max-count={HISTORY_LIMIT}"),
            "--format=%H%x00%h%x00%an%x00%ad%x00%s",
            "HEAD",
            "--",
        ])?;
        parse_history(&output)
    }

    pub(super) fn details(&self, hash: &str) -> Result<CommitDetails, String> {
        if !is_object_id(hash) {
            return Err("Invalid commit object ID.".into());
        }
        let message = self.read(&[
            "show",
            "--no-show-signature",
            "--encoding=UTF-8",
            "--no-patch",
            "--format=%B",
            hash,
            "--",
        ])?;
        // A merge is compared with its first parent instead of emitting the
        // combined format, which a two-sided Diff cannot represent.
        let patch = self.read(&[
            "show",
            "--no-show-signature",
            "--no-color",
            "--no-ext-diff",
            "--no-textconv",
            "--format=",
            "--first-parent",
            "--patch",
            "--find-renames",
            "--src-prefix=a/",
            "--dst-prefix=b/",
            hash,
            "--",
        ])?;
        let files = DiffFile::parse(&patch).map_err(|error| error.to_string())?;
        Ok(CommitDetails {
            message: message
                .split_once("\n\n")
                .map(|(_, body)| body.trim_end())
                .unwrap_or("")
                .to_owned()
                .into(),
            files,
        })
    }
}

fn is_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn parse_history(output: &str) -> Result<Vec<Commit>, String> {
    if output.is_empty() {
        return Ok(Vec::new());
    }
    let fields = output
        .strip_suffix('\0')
        .unwrap_or(output)
        .split('\0')
        .collect::<Vec<_>>();
    if fields.len() % 5 != 0 {
        return Err("Git returned an incomplete commit record.".into());
    }
    fields
        .chunks_exact(5)
        .map(|fields| {
            if !is_object_id(fields[0]) {
                return Err("Git returned an invalid commit object ID.".into());
            }
            Ok(Commit {
                hash: fields[0].to_owned().into(),
                short_hash: fields[1].to_owned().into(),
                author: fields[2].to_owned().into(),
                date: fields[3].to_owned().into(),
                subject: fields[4].to_owned().into(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_records_keep_unicode_tabs_and_empty_subjects() {
        let first = "a".repeat(40);
        let second = "b".repeat(64);
        let output = format!(
            "{first}\0aaaaaaa\0李明\02026-10-07\0Fix tabs\tand Unicode 🦀\0{second}\0bbbbbbb\0Another author\02026-10-06\0\0"
        );
        let commits = parse_history(&output).unwrap();
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].author.as_str(), "李明");
        assert_eq!(commits[0].subject.as_str(), "Fix tabs\tand Unicode 🦀");
        assert!(commits[1].subject.is_empty());
        assert_eq!(commits[1].hash.len(), 64);
        assert!(parse_history("").unwrap().is_empty());
    }

    #[test]
    fn incomplete_records_and_option_like_hashes_are_rejected() {
        assert!(parse_history("hash\0short\0author").is_err());
        assert!(parse_history("--output=somewhere\0short\0author\0date\0subject\0").is_err());
        assert!(!is_object_id(&"g".repeat(40)));
        assert!(!is_object_id(&"a".repeat(39)));
    }
}
