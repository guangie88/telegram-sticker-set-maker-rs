use emojicons::EmojiFormatter;
use serde::de::{Deserialize, Deserializer, Error};
use std::char;
use std::collections::HashMap;
use std::ops::Deref;
use std::path::PathBuf;
use std::string::ToString;

#[derive(StructOpt, Debug)]
#[structopt(name = "telegram-sticker-set-maker-conf")]
/// Configuration for telegram-sticker-set-maker
pub struct Conf {
    /// Input directory with the 512 px sized PNG image files
    #[structopt(parse(from_os_str))]
    pub indir: PathBuf,

    #[structopt(
        short = "a",
        long = "auth",
        default_value = ".telegram-auth.toml",
        parse(from_os_str)
    )]
    /// File path to required Telegram authentication values
    #[structopt(parse(from_os_str))]
    pub auth_path: PathBuf,

    #[structopt(short = "g", long = "glob", default_value = "*.png")]
    /// Glob pattern to get PNG image files
    pub glob: String,

    #[structopt(short = "n", long = "name")]
    /// Sticker set name without bot suffix.
    /// Must not contain spaces and must be unique.
    pub sticker_set_name: String,

    #[structopt(short = "t", long = "title")]
    /// Sticker set title, human-reading friendly name of the sticker set
    pub sticker_set_title: String,

    #[structopt(long = "default-emoji", default_value = "😄")]
    /// Default emoji to assign to every sticker in the set
    pub default_emoji: String,

    #[structopt(short = "e", long = "emoji-mapping", parse(from_os_str))]
    /// Emoji mapping file to use. If not provided, all stickers will use the
    /// default emoji
    pub emoji_mapping: Option<PathBuf>,

    #[structopt(short = "v", parse(from_occurrences))]
    /// Verbose flag (-v, -vv, -vvv)
    pub verbose: u8,
}

#[derive(Debug, Deserialize)]
/// Configuration for Telegram required authentication values
pub struct Auth {
    /// Bot token whose format is <BOT_ID>:<REST_OF_TOKEN>
    pub bot_token: String,

    /// Bot name, used for sticker set suffix
    pub bot_name: String,

    /// User ID to put the new sticker set under
    pub user_id: String,
}

#[derive(Debug)]
/// Wrapper type over unicode char to represent emoji
pub struct Emoji(char);

/// Implements Deref trait for Emoji
impl Deref for Emoji {
    type Target = char;

    /// Gets the unicode char in Emoji
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Implements ToString trait for Emoji
impl ToString for Emoji {
    /// Formats emoji unicode char into String
    fn to_string(&self) -> String {
        format!("{}", self.0)
    }
}

/// Implements conversion from string name into the Emoji char for
/// deserialization
impl<'de> Deserialize<'de> for Emoji {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer).and_then(|s| {
            format!("{}", EmojiFormatter(&format!(":{}:", s)))
                .parse::<char>()
                .map(Emoji)
                .map_err(D::Error::custom)
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
/// Describes either single emoji or multiple emojis mapping
pub enum EmojiMapping {
    /// Single emoji mapping variant
    Single(Emoji),

    /// Multiple emojis mapping variant
    Multi(Vec<Emoji>),
}

/// Sticker to emoji / emojis mapping alias type
pub type StickerEmojiMapping = HashMap<String, EmojiMapping>;
