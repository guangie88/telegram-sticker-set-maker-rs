use failure;
use glob::glob;
use regex::Regex;
use reqwest::ClientBuilder;
use std::{self, fs::File, io::Read, path::Path};
use telepathy::{
    stickers::{Client, StickerFile}, util::Emoji,
};
use toml;
use vec1::Vec1;

use structs::{AddConf, Auth, StickerEmojiMapping};

type Result<T> = std::result::Result<T, failure::Error>;

// other constants
const BOT_TOKEN_SPLIT_RE: &str = r"(\d+):(.+)";

/// Returns at least one emoji (one emoji is one unicode char).
fn get_mapped_emojis<P>(
    file_path: P,
    mapping: Option<&StickerEmojiMapping>,
    default_emoji: Emoji,
) -> Result<Vec1<Emoji>>
where
    P: AsRef<Path>,
{
    let file_name = file_path.as_ref().file_name();

    let emoji_mapping = file_name
        .and_then(|file_name| mapping.map(|mapping| (file_name, mapping)))
        .and_then(|(file_name, mapping)| {
            mapping.get(file_name.to_string_lossy().as_ref())
        });

    let emojis = match emoji_mapping {
        Some(emojis) => emojis.clone(),
        None => vec1![default_emoji],
    };

    Ok(emojis)
}

pub fn add(add_conf: &AddConf) -> Result<()> {
    if !add_conf.indir.exists() {
        Err(format_err!(
            "{:?} input directory does not exists!",
            &add_conf.indir
        ))?
    }

    // get emoji mapping if available
    let emoji_mapping: Option<StickerEmojiMapping> =
        if let Some(ref mapping_path) = add_conf.emoji_mapping {
            let mut mapping_file = File::open(mapping_path)?;
            let mut content = String::new();
            mapping_file.read_to_string(&mut content)?;
            Some(toml::from_str(&content)?)
        } else {
            None
        };

    let merged_glob = {
        let mut indir = add_conf.indir.clone();
        indir.push(&add_conf.glob);
        indir.to_string_lossy().to_string()
    };

    // gets all the image files
    let paths = glob(&merged_glob)?;

    let mut image_paths: Vec<_> = paths
        .filter_map(|path| path.ok())
        .filter(|path| !path.is_dir())
        .collect();

    image_paths.sort();

    // get all the required authentication values
    let auth_content = {
        let mut auth_content = String::new();
        let mut f = File::open(&add_conf.auth_path)?;
        f.read_to_string(&mut auth_content)?;
        auth_content
    };

    let auth: Auth = toml::from_str(&auth_content)?;
    let bot_token_split_re = Regex::new(BOT_TOKEN_SPLIT_RE)?;

    if !bot_token_split_re.is_match(&auth.bot_token) {
        Err(format_err!("Bot token is not of correct format!"))?;
    }

    if let Some((first_path, rest_paths)) = image_paths.split_first() {
        // disable timeout
        let mut builder = ClientBuilder::new();
        builder.timeout(None);
        let client = builder.build()?;

        let client = Client {
            client,
            token: auth.bot_token.clone(),
            user_id: auth.user_id.clone(),
            default_emoji: add_conf.default_emoji,
        };

        let sticker_set_name =
            format!("{}_by_{}", add_conf.sticker_set_name, auth.bot_name);

        {
            // first image must create the new sticker set
            v1!("Sending first image {:?}...", first_path);

            let emojis = get_mapped_emojis(
                &first_path,
                emoji_mapping.as_ref(),
                add_conf.default_emoji,
            )?;

            client.create_new_sticker_set(
                &sticker_set_name,
                &add_conf.sticker_set_title,
                &StickerFile::LocalFile(first_path.clone()),
                &emojis,
                None,
                &None,
            )?;
        }

        for rest_path in rest_paths {
            v1!("Sending image {:?}...", rest_path);

            let emojis = get_mapped_emojis(
                &rest_path,
                emoji_mapping.as_ref(),
                add_conf.default_emoji,
            )?;

            client.add_sticker_to_set(
                &sticker_set_name,
                &StickerFile::LocalFile(rest_path.clone()),
                &emojis,
                &None,
            )?;
        }

        v1!(
            "New sticker set name: \"{}\", title: {}, # of stickers: {}",
            sticker_set_name,
            &add_conf.sticker_set_title,
            image_paths.len(),
        );

        v0!(
            "Click in Telegram to add: https://t.me/addstickers/{}",
            sticker_set_name
        );
    } else {
        v0!("No stickers found, no sticker set created!");
    }

    Ok(())
}
