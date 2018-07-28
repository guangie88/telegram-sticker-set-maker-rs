#![cfg_attr(feature = "cargo-clippy", deny(clippy))]
#![deny(missing_debug_implementations)]

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
extern crate telepathy;
extern crate toml;
extern crate url;
#[macro_use]
extern crate vec1;
#[macro_use]
extern crate vlog;

use structopt::StructOpt;

mod add;
mod structs;

use add::add;
use structs::{Command, Conf};

type Result<T> = std::result::Result<T, failure::Error>;

fn run(conf: &Conf) -> Result<()> {
    vlog::set_verbosity_level(conf.verbose as usize);

    match conf.cmd {
        Command::Add(ref add_conf) => add(add_conf),
    }
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
    use structs::StickerEmojiMapping;

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
            "1192266.png" = ["wink"]
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
            "1192266.png" = ["wink"]
            "1192267.png" = ["blush", "relaxed"]
        "#;

        let mapping = toml::from_str::<StickerEmojiMapping>(CONTENT);
        assert!(mapping.is_ok());
    }

    #[test]
    fn test_emoji_map_invalid_1() {
        const CONTENT: &str = r#"
            "1192266.png" = []
        "#;

        let mapping = toml::from_str::<StickerEmojiMapping>(CONTENT);
        assert!(mapping.is_err());
    }
}
