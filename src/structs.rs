use std::collections::HashMap;
use std::path::PathBuf;
use std::string::ToString;
use vec1::Vec1;

use telepathy::util::Emoji;

#[derive(StructOpt, Debug)]
/// Command to run
pub enum Command {
    #[structopt(name = "add")]
    /// Add a new sticker set with stickers
    Add(AddConf),
}

#[derive(StructOpt, Debug)]
/// Add configuration values
pub struct AddConf {
    #[structopt(parse(from_os_str))]
    /// Input directory with the 512 px sized PNG image files
    pub indir: PathBuf,

    #[structopt(
        short = "a",
        long = "auth",
        default_value = ".telegram-auth.toml",
        parse(from_os_str)
    )]
    /// File path to required Telegram authentication values
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

    #[structopt(long = "default-emoji", default_value = "smile")]
    /// Default emoji to assign to every sticker in the set
    pub default_emoji: Emoji,

    #[structopt(short = "e", long = "emoji-mapping", parse(from_os_str))]
    /// Emoji mapping file to use. If not provided, all stickers will use the
    /// default emoji
    pub emoji_mapping: Option<PathBuf>,
}

#[derive(StructOpt, Debug)]
#[structopt(name = "telegram-sticker-set-maker-conf")]
/// Configuration for telegram-sticker-set-maker
pub struct Conf {
    #[structopt(subcommand)]
    /// Command to run
    pub cmd: Command,

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

/// Sticker to emoji / emojis mapping alias type
pub type StickerEmojiMapping = HashMap<String, Vec1<Emoji>>;
