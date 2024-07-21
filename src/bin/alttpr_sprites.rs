use std::{
    fs::{self, create_dir_all},
    io::{Cursor, Read as _},
    path::Path,
};

use image::{
    io::Reader, DynamicImage, GenericImage, GenericImageView, ImageFormat, Pixel, Rgb, Rgba,
};
use imageproc::drawing::text_size;
use reqwest::blocking::Client;

use crate::sprites::get_sprites_metadata;
use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)]
    username: String,
}

fn main() -> anyhow::Result<()> {
    println!("hello");
    let args = Args::parse();
    let c = reqwest::blocking::Client::builder().build()?;
    let metadata = get_sprites_metadata(&c, &args)?;
    for m in metadata
        .iter()
        .filter(|s| s.name.to_lowercase().contains("link"))
    {
        if let Err(e) = m.upscale_sprite(&c) {
            println!("Error fetching {}: {e}", m.name);
        }
    }
    Ok(())
}

#[derive(serde::Deserialize, Debug)]
struct Sprite {
    name: String,
    #[cfg(feature = "rich_sprites")]
    author: String,
    #[cfg(feature = "rich_sprites")]
    version: u32,
    #[cfg(feature = "rich_sprites")]
    file: String,
    preview: String,
    #[cfg(feature = "rich_sprites")]
    tags: Vec<String>,
    #[cfg(feature = "rich_sprites")]
    usage: Vec<String>,
}

impl Sprite {
    fn preview_filename(&self) -> &str {
        self.preview.split('/').last().unwrap()
    }

    // these are 16x24
    fn get_preview_data(&self, c: &Client) -> anyhow::Result<Vec<u8>> {
        let cache_path = Path::new("./sprites").join(&self.preview_filename());
        if cache_path.exists() {
            Ok(fs::read(cache_path)?)
        } else {
            let req = c.get(&self.preview).build()?;
            let mut resp = c.execute(req)?;
            if !resp.status().is_success() {
                return Err(anyhow::anyhow!(
                    "Error fetching sprite preview: {}",
                    resp.status()
                ));
            }
            let mut buf = Vec::<u8>::with_capacity(resp.content_length().unwrap_or(69) as usize);
            resp.read_to_end(&mut buf)?;
            std::fs::write(cache_path, &buf)
                .map_err(|e| anyhow::anyhow!("Error caching preview data: {e}"))?;
            Ok(buf)
        }
    }

    fn upscale_sprite(&self, c: &Client) -> anyhow::Result<()> {
        let img_data = self.get_preview_data(c)?;
        let c = Cursor::new(img_data.as_slice());
        let ir = Reader::with_format(c, ImageFormat::Png);
        let image = ir.decode()?;
        if (image.width(), image.height()) != (16, 24) {
            return Err(anyhow::anyhow!(
                "Error handling {}: expected 16x24 image size",
                self.name
            ));
        }
        let mut opaque = DynamicImage::new(24, 24, image.color());
        for y in 0..24 {
            for x in 0..4 {
                opaque.put_pixel(x, y, Rgba([255, 255, 255, 255]));
                opaque.put_pixel(24 - x - 1, y, Rgba([255, 255, 255, 255]));
            }
        }
        for (x, y, mut rgba) in image.pixels() {
            if let Some(alpha) = rgba.0.get(3) {
                if *alpha == 0 {
                    rgba[0] = u8::MAX;
                    rgba[1] = u8::MAX;
                    rgba[2] = u8::MAX;
                }
                rgba[3] = u8::MAX;
            }
            opaque.put_pixel(x + 4, y, rgba);
        }
        let mut bigger = opaque.resize(16 * 16, 24 * 16, image::imageops::FilterType::Nearest);
        let base_path = Path::new(self.preview_filename());
        let embiggened_path_str = format!(
            "{}_big.{}",
            base_path.file_stem().unwrap().to_str().unwrap(),
            base_path.extension().unwrap().to_str().unwrap(),
        );
        let big_dir = Path::new("./sprites/big/");
        let embiggened_path = big_dir.join(&embiggened_path_str);

        // ok and lets write the name on it
        let font = ab_glyph::FontRef::try_from_slice(include_bytes!("../../fonts/ARIALBD.TTF"))?;
        let h = bigger.height() as i32;

        // returns (x offset, font scale)
        let situate_text = |width: u32, text: &str| -> anyhow::Result<(i32, f32)> {
            for trial_size in [64.0, 56.0, 48.0, 32.0, 24.0] {
                let (w, _) = text_size(trial_size, &font, text);
                if w < width {
                    let offset = (width - w) / 2;

                    return Ok((offset as i32, trial_size));
                }
            }
            Err(anyhow::anyhow!("Could not find a good size for this text"))
        };

        let (w, s) = situate_text(bigger.width(), &self.name)?;
        imageproc::drawing::draw_text_mut(
            &mut bigger,
            Rgba::from([222, 32, 32, 255]),
            w,
            h - s as i32 - 4,
            s,
            &font,
            &self.name,
        );

        create_dir_all(big_dir)?;
        bigger.save(embiggened_path)?;
        Ok(())
    }
}

mod sprites {
    use std::{
        fs::{create_dir_all, read_to_string, remove_file},
        path::{Path, PathBuf},
    };

    use reqwest::blocking::Client;

    use crate::{Args, Sprite};

    pub fn get_sprites_metadata(c: &Client, args: &Args) -> anyhow::Result<Vec<Sprite>> {
        let raw = if let Some(r) = get_sprites_from_filesystem(args)? {
            println!("Got sprites from filesystem cache");
            r
        } else if let Some(r) = get_sprites_from_api(c)? {
            println!("Got sprites from API");
            r
        } else {
            return Err(anyhow::anyhow!("Could not find sprite metadata"));
        };
        let cached_path = Path::new("./sprites/sprites.json");
        let parsed = match serde_json::from_str::<Vec<Sprite>>(&raw) {
            Ok(parsed) => {
                std::fs::write(cached_path, &raw).ok();
                parsed
            }
            Err(e) => {
                remove_file(cached_path).ok();
                return Err(anyhow::anyhow!("Error parsing sprites json: {e}"));
            }
        };

        Ok(parsed)
    }

    fn get_sprites_from_filesystem(args: &Args) -> anyhow::Result<Option<String>> {
        let p = PathBuf::from(format!("./sprites_{}", args.username));
        create_dir_all(&p)?;
        let s = p.join("sprites.json");
        // TODO: cache bust
        if s.exists() {
            // N.B. i know this is inefficient i just dont care that much
            Ok(Some(read_to_string(s)?))
        } else {
            Ok(None)
        }
    }

    fn get_sprites_from_api(c: &Client, args: &Args) -> anyhow::Result<Option<String>> {
        let req = c.get("https://alttpr.com/sprites").build()?;
        let resp = c.execute(req)?;

        let body = std::io::read_to_string(resp)?;
        Ok(Some(body))
    }
}
