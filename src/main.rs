#![cfg_attr(feature = "cargo-clippy", deny(clippy))]
#![deny(missing_debug_implementations, warnings)]

extern crate emojicons;
#[macro_use]
extern crate failure;
extern crate glob;
extern crate regex;
extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate structopt;
#[macro_use]
extern crate structopt_derive;
extern crate toml;
extern crate url;
#[macro_use]
extern crate vlog;

mod structs;

use glob::glob;
use regex::Regex;
use reqwest::multipart::Form;
use reqwest::{ClientBuilder, RequestBuilder, StatusCode, Url};
use std::fs::File;
use std::io::Read;
use std::ops::Deref;
use std::path::Path;
use structopt::StructOpt;
use structs::{Auth, Conf, EmojiMapping, StickerEmojiMapping};

type Result<T> = std::result::Result<T, failure::Error>;

// endpoints
const ADD_STICKER_TO_SET_EP: &str = "/addStickerToSet";
const CREATE_NEW_STICKER_SET_EP: &str = "/createNewStickerSet";

// other constants
const BOT_TOKEN_SPLIT_RE: &str = r"(\d+):(.+)";
const TELEGRAM_BOT_API: &str = "https://api.telegram.org/bot";

fn add_png_sticker<P>(req: &mut RequestBuilder, image_path: P) -> Result<()>
where
    P: AsRef<Path>,
{
    let form = Form::new().file("png_sticker", image_path.as_ref())?;
    req.multipart(form);
    Ok(())
}

/// Returns at least one emoji (one emoji is one unicode char).
fn get_emojis<P>(
    file_path: P,
    mapping: Option<&StickerEmojiMapping>,
    default_emoji: &str,
) -> String
where
    P: AsRef<Path>,
{
    let file_name = file_path.as_ref().file_name();

    let emoji_mapping = file_name
        .and_then(|file_name| mapping.map(|mapping| (file_name, mapping)))
        .and_then(|(file_name, mapping)| {
            mapping.get(file_name.to_string_lossy().as_ref())
        });

    match emoji_mapping {
        Some(&EmojiMapping::Single(ref emoji)) => emoji.to_string(),
        Some(&EmojiMapping::Multi(ref emojis)) => {
            let mut s = String::new();

            for emoji in emojis {
                s.push(*emoji.deref())
            }

            s
        }
        None => default_emoji.to_owned(),
    }
}

fn run(conf: &Conf) -> Result<()> {
    vlog::set_verbosity_level(conf.verbose as usize);

    if !conf.indir.exists() {
        Err(format_err!(
            "{:?} input directory does not exists!",
            &conf.indir
        ))?
    }

    // get emoji mapping if available
    let emoji_mapping: Option<StickerEmojiMapping> =
        if let Some(ref mapping_path) = conf.emoji_mapping {
            let mut mapping_file = File::open(mapping_path)?;
            let mut content = String::new();
            mapping_file.read_to_string(&mut content)?;
            Some(toml::from_str(&content)?)
        } else {
            None
        };

    let merged_glob = {
        let mut indir = conf.indir.clone();
        indir.push(&conf.glob);
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
        let mut f = File::open(&conf.auth_path)?;
        f.read_to_string(&mut auth_content)?;
        auth_content
    };

    let auth: Auth = toml::from_str(&auth_content)?;
    let bot_token_split_re = Regex::new(BOT_TOKEN_SPLIT_RE)?;

    if !bot_token_split_re.is_match(&auth.bot_token) {
        Err(format_err!(
            "Bot token is not of correct format!"
        ))?;
    }

    // get all the endpoints ready
    let create_new_sticker_set_url = Url::parse(&format!(
        "{}{}{}",
        TELEGRAM_BOT_API, auth.bot_token, CREATE_NEW_STICKER_SET_EP
    ))?;

    let add_sticker_to_set_url = Url::parse(&format!(
        "{}{}{}",
        TELEGRAM_BOT_API, auth.bot_token, ADD_STICKER_TO_SET_EP
    ))?;

    if let Some((first_path, rest_paths)) = image_paths.split_first() {
        // disable timeout
        let mut builder = ClientBuilder::new();
        builder.timeout(None);
        let client = builder.build()?;

        let sticker_set_name =
            format!("{}_by_{}", conf.sticker_set_name, auth.bot_name);

        {
            // first image must create the new sticker set
            v1!("Sending first image {:?}...", first_path);
            let mut req = client.get(create_new_sticker_set_url);
            add_png_sticker(&mut req, &first_path)?;

            req.query(&[
                ("user_id", &auth.user_id),
                ("name", &sticker_set_name),
                ("title", &conf.sticker_set_title),
                (
                    "emojis",
                    &get_emojis(
                        &first_path,
                        emoji_mapping.as_ref(),
                        &conf.default_emoji,
                    ),
                ),
            ]);

            v3!("{:?}", req);
            let mut resp = req.send()?;

            v1!(
                "Status: {}, msg: {}",
                resp.status(),
                resp.text()?
            );

            if resp.status() != StatusCode::Ok {
                Err(format_err!(
                    "First image status: {:?}, not continuing...",
                    resp.status()
                ))?;
            }
        }

        for rest_path in rest_paths {
            v1!("Sending image {:?}...", rest_path);
            let mut req = client.get(add_sticker_to_set_url.clone());
            add_png_sticker(&mut req, &rest_path)?;

            req.query(&[
                ("user_id", &auth.user_id),
                ("name", &sticker_set_name),
                (
                    "emojis",
                    &get_emojis(
                        &rest_path,
                        emoji_mapping.as_ref(),
                        &conf.default_emoji,
                    ),
                ),
            ]);

            v3!("{:?}", req);

            let mut resp = req.send()?;

            v1!(
                "Status: {}, msg: {}",
                resp.status(),
                resp.text()?
            );

            if resp.status() != StatusCode::Ok {
                Err(format_err!(
                    "Image status: {:?}, not continuing...",
                    resp.status()
                ))?;
            }
        }

        v1!(
            "New sticker set name: \"{}\", title: {}, # of stickers: {}",
            sticker_set_name,
            &conf.sticker_set_title,
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

fn main() {
    let conf = Conf::from_args();

    match run(&conf) {
        Ok(_) => v1!("telegram-sticker-set-maker COMPLETED!"),
        Err(e) => ve0!("{}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emoji_map_valid_1() {
        const CONTENT: &str = "";
        let mapping = toml::from_str::<StickerEmojiMapping>(CONTENT);
        assert!(mapping.is_ok());
    }

    #[test]
    fn test_emoji_map_valid_2() {
        const CONTENT: &str = r#"
            "1192266.png" = "wink"
        "#;

        let mapping = toml::from_str::<StickerEmojiMapping>(CONTENT);
        assert!(mapping.is_ok());
    }

    #[test]
    fn test_emoji_map_valid_3() {
        const CONTENT: &str = r#"
            "1192267.png" = ["blush", "relaxed"]
        "#;

        let mapping = toml::from_str::<StickerEmojiMapping>(CONTENT);
        assert!(mapping.is_ok());
    }

    #[test]
    fn test_emoji_map_valid_4() {
        const CONTENT: &str = r#"
            "1192266.png" = "wink"
            "1192267.png" = ["blush", "relaxed"]
        "#;

        let mapping = toml::from_str::<StickerEmojiMapping>(CONTENT);
        assert!(mapping.is_ok());
    }
}
