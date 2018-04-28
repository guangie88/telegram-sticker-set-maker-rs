#![cfg_attr(feature = "cargo-clippy", deny(clippy))]
#![deny(missing_debug_implementations, warnings)]

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

use glob::glob;
use regex::Regex;
use reqwest::multipart::Form;
use reqwest::{Client, RequestBuilder, StatusCode, Url};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

type Result<T> = std::result::Result<T, failure::Error>;

const TELEGRAM_BOT_API: &str = "https://api.telegram.org/bot";
const CREATE_NEW_STICKER_SET_EP: &str = "/createNewStickerSet";
const ADD_STICKER_TO_SET_EP: &str = "/addStickerToSet";

const BOT_TOKEN_SPLIT_RE: &str = r"(\d+):(.+)";

#[derive(Deserialize)]
/// Configuration for Telegram required authentication values
struct Auth {
    /// Bot token whose format is <BOT_ID>:<REST_OF_TOKEN>
    bot_token: String,

    /// Bot name, used for sticker set suffix
    bot_name: String,

    /// User ID to put the new sticker set under
    user_id: String,
}

#[derive(StructOpt, Debug)]
#[structopt(name = "telegram-sticker-set-maker-conf")]
/// Configuration for telegram-sticker-set-maker
struct Conf {
    /// Input directory with the 512 px sized PNG image files
    #[structopt(parse(from_os_str))]
    indir: PathBuf,

    #[structopt(
        short = "a",
        long = "auth",
        default_value = ".telegram-auth.toml",
        parse(from_os_str)
    )]
    /// File path to required Telegram authentication values
    auth_path: PathBuf,

    #[structopt(short = "g", long = "glob", default_value = "*.png")]
    /// Glob pattern to get PNG image files
    glob: String,

    #[structopt(short = "n", long = "name")]
    /// Sticker set name without bot suffix.
    /// Must not contain spaces and must be unique.
    sticker_set_name: String,

    #[structopt(short = "t", long = "title")]
    /// Sticker set title, human-reading friendly name of the sticker set
    sticker_set_title: String,

    #[structopt(short = "e", long = "emoji", default_value = "😄")]
    /// Emoji to assign to every sticker in the set
    emoji: String,

    #[structopt(short = "v", parse(from_occurrences))]
    /// Verbose flag (-v, -vv, -vvv)
    verbose: u8,
}

fn add_png_sticker<P>(req: &mut RequestBuilder, image_path: P) -> Result<()>
where
    P: AsRef<Path>,
{
    let form = Form::new().file("png_sticker", image_path.as_ref())?;
    req.multipart(form);
    Ok(())
}

fn run(conf: &Conf) -> Result<()> {
    vlog::set_verbosity_level(conf.verbose as usize);

    if !conf.indir.exists() {
        Err(format_err!(
            "{:?} input directory does not exists!",
            &conf.indir
        ))?
    }

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
        let client = Client::new();

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
                ("emojis", &conf.emoji),
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
                ("emojis", &conf.emoji),
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
            "Created sticker set name: \"{}\", title: {}, # of stickers: {}",
            sticker_set_name,
            &conf.sticker_set_title,
            image_paths.len(),
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
