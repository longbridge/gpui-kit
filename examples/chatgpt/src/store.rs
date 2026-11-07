//! Chat history and projects, saved as JSON next to the sign-in under the
//! platform config directory. Images are stored as files beside them.

use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context as _, Result};
use rand::RngCore as _;
use serde::{Deserialize, Serialize};

/// A folder of conversations that share instructions, like ChatGPT projects.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub instructions: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StoredRole {
    User,
    Assistant,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredSource {
    pub title: String,
    pub url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredMessage {
    pub role: StoredRole,
    pub text: String,
    #[serde(default)]
    pub reasoning: String,
    #[serde(default)]
    pub thought_for_ms: Option<u64>,
    #[serde(default)]
    pub searches: Vec<String>,
    #[serde(default)]
    pub sources: Vec<StoredSource>,
    /// File names under the images directory.
    #[serde(default)]
    pub images: Vec<String>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub project_id: Option<String>,
    /// Unix seconds of the last message.
    pub updated_at: u64,
    pub messages: Vec<StoredMessage>,
}

impl Conversation {
    pub fn new(project_id: Option<String>) -> Self {
        Self {
            id: new_id(),
            title: String::new(),
            project_id,
            updated_at: now(),
            messages: Vec::new(),
        }
    }
}

/// Reads and writes the history directory. Every call is synchronous and
/// small; a conversation is rewritten whole when it changes.
pub struct Store {
    root: PathBuf,
}

impl Store {
    pub fn open() -> Result<Self> {
        let root = dirs::config_dir()
            .context("no config directory")?
            .join("gpui-kit-chatgpt");
        std::fs::create_dir_all(root.join("chats"))?;
        std::fs::create_dir_all(root.join("images"))?;
        Ok(Self { root })
    }

    pub fn projects(&self) -> Vec<Project> {
        std::fs::read(self.root.join("projects.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub fn save_projects(&self, projects: &[Project]) -> Result<()> {
        std::fs::write(
            self.root.join("projects.json"),
            serde_json::to_vec_pretty(projects)?,
        )?;
        Ok(())
    }

    /// Every saved conversation, most recent first.
    pub fn conversations(&self) -> Vec<Conversation> {
        let mut conversations: Vec<Conversation> = std::fs::read_dir(self.root.join("chats"))
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| std::fs::read(entry.path()).ok())
            .filter_map(|bytes| serde_json::from_slice(&bytes).ok())
            .collect();
        conversations.sort_by_key(|conversation| std::cmp::Reverse(conversation.updated_at));
        conversations
    }

    pub fn save_conversation(&self, conversation: &Conversation) -> Result<()> {
        std::fs::write(
            self.chat_path(&conversation.id),
            serde_json::to_vec_pretty(conversation)?,
        )?;
        Ok(())
    }

    pub fn delete_conversation(&self, conversation: &Conversation) {
        _ = std::fs::remove_file(self.chat_path(&conversation.id));
        for message in &conversation.messages {
            for image in &message.images {
                _ = std::fs::remove_file(self.image_path(image));
            }
        }
    }

    /// Save image bytes and return the file name to reference them by.
    pub fn save_image(&self, bytes: &[u8], extension: &str) -> Result<String> {
        let name = format!("{}.{extension}", new_id());
        std::fs::write(self.image_path(&name), bytes)?;
        Ok(name)
    }

    pub fn image_path(&self, name: &str) -> PathBuf {
        self.root.join("images").join(name)
    }

    fn chat_path(&self, id: &str) -> PathBuf {
        self.root.join("chats").join(format!("{id}.json"))
    }
}

pub fn new_id() -> String {
    let mut bytes = [0u8; 8];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

/// A conversation's title: its first line, shortened at a word boundary.
pub fn title_from(text: &str) -> String {
    const MAX: usize = 48;
    let line = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .trim();
    if line.chars().count() <= MAX {
        return line.to_string();
    }
    let cut: String = line.chars().take(MAX).collect();
    let cut = cut.rsplit_once(' ').map_or(cut.as_str(), |(head, _)| head);
    format!("{}…", cut.trim_end())
}

/// Which sidebar section a conversation updated at `updated_at` belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Age {
    Today,
    Yesterday,
    PreviousWeek,
    Older,
}

impl Age {
    pub fn of(updated_at: u64, now: u64) -> Self {
        const DAY: u64 = 24 * 60 * 60;
        let today_start = now - now % DAY;
        match updated_at {
            t if t >= today_start => Self::Today,
            t if t >= today_start.saturating_sub(DAY) => Self::Yesterday,
            t if t >= today_start.saturating_sub(7 * DAY) => Self::PreviousWeek,
            _ => Self::Older,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Today => "Today",
            Self::Yesterday => "Yesterday",
            Self::PreviousWeek => "Previous 7 days",
            Self::Older => "Older",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_are_the_first_line_shortened_at_a_word() {
        assert_eq!(title_from("\nHello there\nsecond"), "Hello there");
        let long = "Explain how the borrow checker decides when a reference outlives its owner";
        let title = title_from(long);
        assert!(title.ends_with('…'));
        assert!(title.chars().count() <= 49);
        assert!(long.starts_with(title.trim_end_matches('…')));
    }

    #[test]
    fn ages_group_by_calendar_day() {
        const DAY: u64 = 86_400;
        let now = 100 * DAY + 3_600;
        assert_eq!(Age::of(now, now), Age::Today);
        assert_eq!(Age::of(100 * DAY - 1, now), Age::Yesterday);
        assert_eq!(Age::of(95 * DAY, now), Age::PreviousWeek);
        assert_eq!(Age::of(10 * DAY, now), Age::Older);
    }
}
