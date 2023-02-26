use anyhow::{Result, anyhow};
use std::{fs::File, path::{Path, PathBuf}};
use std::time::Instant;
use std::io::{BufReader, BufWriter};
use ril::prelude::*;
use crate::types::*;

const ACCENT_COLOR: Rgb = Rgb::new(241, 143, 243);
//const GRAY: Rgb = Rgb::new(163, 163, 176);

const ANIME_POSTER_FULLRES_FOLDER: &str = "fullres";

const ANIME_POSTER_MEDIUM_FOLDER: &str = "310x468";
const ANIME_POSTER_MEDIUM_WIDTH: u32 = 310;
const ANIME_POSTER_MEDIUM_HEIGHT: u32 = 468;

const ANIME_PRESENTER_TEMPLATE: &str = "assets/templates/AnimePresenter.png";
const ANIME_PRESENTER_FOLDER: &str = "pre";

const ANIME_PLACEHOLDER_COMPONENTS_X: u32 = 4;
const ANIME_PLACEHOLDER_COMPONENTS_Y: u32 = 7;
const DIGIT: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz#$%*+,-.:;=?@[]^_{|}~";

fn decode83(s: &str, start: usize, end: usize) -> usize {
    let mut value = 0;
    for c in s.chars().skip(start).take(end-start) {
        value *= 83;
        value += DIGIT.find(c).expect("invalid char");
    }
    return value;
}

pub fn get_fullres_path(key: &str, folder: &Path) -> PathBuf {
    folder.join(ANIME_POSTER_FULLRES_FOLDER).join(format!("{key}.webp"))
}

pub fn export_poster(cache_key: String, from: &Path, folder: &Path) -> Result<CachedImage> {
    let file_name: String = format!("{cache_key}.webp");
    let mut image: Image<Rgb> = Image::from_reader(ImageFormat::WebP, BufReader::new(File::open(from)?))
        .map_err(|e| anyhow!("Unable to open uploaded file: {e:?}"))?;

    // original poster
    let output = folder.join(ANIME_POSTER_FULLRES_FOLDER).join(file_name.clone());
    image.encode(ImageFormat::WebP, &mut BufWriter::new(File::create(output)?))
        .map_err(|e| anyhow!("Unable to save original image: {e:?}"))?;

    // small poster
    image.resize(ANIME_POSTER_MEDIUM_WIDTH, ANIME_POSTER_MEDIUM_HEIGHT, ResizeAlgorithm::Lanczos3);
    let output = folder.join(ANIME_POSTER_MEDIUM_FOLDER).join(file_name);
    image.encode(ImageFormat::WebP, &mut BufWriter::new(File::create(output)?))
        .map_err(|e| anyhow!("Unable to save resized image: {e:?}"))?;

    let placeholder = {
        let rgba: Vec<u8> = image.pixels().flatten().map(|p| [p.r, p.g, p.b, 255]).flatten().collect();
        blurhash::encode(ANIME_PLACEHOLDER_COMPONENTS_X, ANIME_PLACEHOLDER_COMPONENTS_Y, 
            image.width(), image.height(), &rgba)
    };
    Ok(CachedImage::with_placeholder(cache_key, placeholder))
}

pub fn export_presenter<T: AsRef<AnimeSeries>>(recipient: T, from: &Path, folder: &Path) -> Result<()> {
    let recipient: &AnimeSeries = recipient.as_ref();
    let key = recipient.poster.key().to_string();
    let file_name: String = format!("{key}.webp");
    let avg_color = match recipient.poster.placeholder() {
        Some(blurhash) => {
            let avg_color = decode83(blurhash, 2, 6);
            Rgb::new((avg_color >> 16) as u8, (avg_color >> 8) as u8, avg_color as u8)
        },
        None => ACCENT_COLOR
    };

    let (mut presenter, poster_width) = {
        let mut template: Image<Rgb> = Image::open(ANIME_PRESENTER_TEMPLATE)
            .map_err(|e| anyhow!("Unable to open template image: {e:?}"))?;
        let input = BufReader::new(File::open(from)?);
        let mut poster: Image<Rgb> = Image::from_reader(ImageFormat::WebP, input)
            .map_err(|e| anyhow!("Unable to open uploaded file: {e:?}"))?;

        let poster_width = ANIME_POSTER_MEDIUM_WIDTH * template.height() / ANIME_POSTER_MEDIUM_HEIGHT;
        poster.resize(poster_width, template.height(), ResizeAlgorithm::Lanczos3);
        template.paste(0, 0, &poster);

        (template, poster_width)
    };

    let bold = Font::open("assets/fonts/Poppins-Bold.ttf", 28.0)
        .map_err(|e| anyhow!("Unable to open font file: {e:?}"))?;
    let xbold = Font::open("assets/fonts/Poppins-ExtraBold.ttf", 64.0)
        .map_err(|e| anyhow!("Unable to open font file: {e:?}"))?;

    presenter.draw(&TextLayout::new() // title
        .with_position(452, 82)
        .with_width(presenter.width() - poster_width - 64)
        .with_wrap(WrapStyle::Word)
        .with_basic_text(&xbold, recipient.titles[0].as_str(), Rgb::white()));

    presenter.draw(&TextLayout::new() // year
        .centered()
        .with_position(452 + 64, 32 + 21 + 2)
        .with_basic_text(&bold, recipient.anime.release_year.to_string(), ACCENT_COLOR));

    let bold = Font::open("assets/fonts/Poppins-Bold.ttf", 32.0)
        .map_err(|e| anyhow!("Unable to open font file: {e:?}"))?;

    presenter.draw(&TextLayout::new() // episode count
        .with_position(532, 534 + 32 + 4)
        .with_vertical_anchor(VerticalAnchor::Center)
        .with_basic_text(&bold, recipient.anime.episodes.to_string(), avg_color)
        .with_basic_text(&bold, " episodes", Rgb::white()));

    presenter.draw(&TextLayout::new() // season count
        .with_position(532, 454 + 32 + 4)
        .with_vertical_anchor(VerticalAnchor::Center)
        .with_basic_text(&bold, recipient.anime.seasons.to_string(), avg_color)
        .with_basic_text(&bold, " seasons", Rgb::white()));

    presenter.draw(&TextLayout::new() // chapter count
        .with_position(532, 374 + 32 + 4)
        .with_vertical_anchor(VerticalAnchor::Center)
        .with_basic_text(&bold, recipient.manga.chapters.to_string(), avg_color)
        .with_basic_text(&bold, " chapters", Rgb::white()));

    presenter.draw(&TextLayout::new() // volume count
        .with_position(532, 294 + 32 + 4)
        .with_vertical_anchor(VerticalAnchor::Center)
        .with_basic_text(&bold, recipient.manga.volumes.to_string(), avg_color)
        .with_basic_text(&bold, " volumes", Rgb::white()));

    let output = folder.join(ANIME_PRESENTER_FOLDER).join(file_name);
    presenter.encode(ImageFormat::WebP, &mut BufWriter::new(File::create(output)?))
        .map_err(|e| anyhow!("Unable to save presenter image: {e:?}"))?;

    Ok(())
}
