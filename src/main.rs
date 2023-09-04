mod find_link;

use crate::find_link::find_link;
use crate::FrameDuration::{Fps30, Fps60};
use anyhow::anyhow;
use image::codecs::gif::{GifEncoder, Repeat};
use image::imageops::{crop, FilterType};
use image::{Delay, DynamicImage, Frame};
use std::cmp::Ordering;
use std::fs::{read_dir, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;
use std::{env, fs};

fn number_from_pathbuf(p: &PathBuf) -> Option<u32> {
    p.file_stem()
        .and_then(|fs| fs.to_str())
        .and_then(|s| u32::from_str(s).ok())
}

fn compare_paths(a: &PathBuf, b: &PathBuf) -> Ordering {
    number_from_pathbuf(&a).cmp(&number_from_pathbuf(&b))
}

fn env_var(k: &str) -> String {
    env::var(k).expect(&format!("Failed to find required environment variable {k}"))
}

fn readdir_to_sorted(path: &str) -> anyhow::Result<Vec<PathBuf>> {
    let mut filenames = read_dir(path)?
        .map(|f| f.unwrap().path())
        .filter(|p| !p.is_dir())
        .collect::<Vec<PathBuf>>();
    filenames.sort_by(compare_paths);
    Ok(filenames)
}

struct ImageSelectionConfig {
    skip: usize,
    take: usize,
    skip_alternating: bool,
}

impl ImageSelectionConfig {
    fn blank() -> Self {
        Self {
            skip: 0,
            take: usize::MAX,
            skip_alternating: false,
        }
    }

    fn from_env() -> anyhow::Result<Self> {
        let skip = env_var("SKIP_IMAGES").parse()?;
        let take = env_var("TAKE_IMAGES").parse()?;
        let skip_alternating = if env_var("SKIP_ALTERNATING").parse::<u32>()? == 1 {
            true
        } else {
            false
        };
        Ok(Self {
            skip,
            take,
            skip_alternating,
        })
    }
}

fn get_images(
    isc: ImageSelectionConfig,
) -> anyhow::Result<impl Iterator<Item = (DynamicImage, PathBuf)>> {
    let dir = env_var("IMAGE_DIR");
    let dir_path = format!("images/{dir}");
    let skip_alternating = isc.skip_alternating;
    let fmap = move |(c, i)| {
        if skip_alternating {
            if c % 2 == 0 {
                Some(i)
            } else {
                None
            }
        } else {
            Some(i)
        }
    };
    let image_paths = readdir_to_sorted(&dir_path)?;
    Ok(image_paths
        .into_iter()
        .skip(isc.skip)
        .enumerate()
        .filter_map(fmap)
        .take(isc.take)
        .map(|f| (image::open(&f).unwrap(), f)))
}

fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    let op = env_var("OPERATION");
    if op == "CROP_AROUND_LINK" {
        crop_around_link()?;
    }
    if op == "MAKE_GIF" {
        let isc = ImageSelectionConfig::from_env()?;
        let fps = if env_var("FPS").parse::<u32>()? == 30 {
            Fps30
        } else {
            Fps60
        };
        let dir = env_var("IMAGE_DIR");
        let out_path = format!("images/out/{dir}.gif");
        let images = get_images(isc)?.map(|(i, p)| i.resize(112, 112, FilterType::Nearest));
        let f = File::create(&out_path)?;
        println!("Writing file to {out_path}");
        write_gif(images, f, fps)?;
    }
    Ok(())
}

fn crop_around_link() -> anyhow::Result<()> {
    let dir = env_var("IMAGE_DIR");
    let out_dir = format!("images/{dir}/link_crops");

    let width: i32 = env_var("CROP_WIDTH").parse()?;
    let height: i32 = env_var("CROP_HEIGHT").parse()?;

    fs::create_dir_all(&out_dir)?;
    for (i, p) in get_images(ImageSelectionConfig::blank())? {
        assert_eq!(i.width(), 256);
        assert_eq!(i.height(), 224);
        match find_link(&i) {
            Some((x, y)) => {
                let mut out_path = PathBuf::from_str(&out_dir)?;
                out_path.push(p.file_name().unwrap());
                // link seems to be 16x24, for context
                // we're getting the top left corner of link
                // to get to `width` pixels, we want to go `width/2` pixels from the middle of 8
                // so that's `(x + 8) - width/2`
                // similarly, height will be `(y + 12) - width/2`
                // and then we need to bound these to the dimensions of the source image
                // which i am assuming here for convenience are 256 x 224
                let mut topleft_x = ((x + 8) - (width as i32 / 2)).clamp(0, 256);
                if topleft_x + width > 256 {
                    topleft_x = (256 - width) as i32;
                }
                let mut topleft_y = ((y + 12) - (height as i32 / 2)).clamp(0, 224);
                if topleft_y + height > 224 {
                    topleft_y = (224 - height) as i32;
                }

                i.crop_imm(
                    topleft_x as u32,
                    topleft_y as u32,
                    width as u32,
                    width as u32,
                )
                .save(&out_path)?;
            }
            None => {
                println!("unable to find link in {p:?}");
            }
        }
    }
    Ok(())
}

fn make_gif_with_crop() -> anyhow::Result<()> {
    let dir = env_var("IMAGE_DIR");
    let out_path = format!("images/out/{dir}.gif");
    let cropper = Cropper::new_from_env()?;
    to_gif(&cropper, &out_path)?;
    Ok(())
}

fn to_gif(cropper: &Cropper, output_fn: &str) -> anyhow::Result<()> {
    let out_width = env_var("OUT_WIDTH").parse()?;
    let out_height = env_var("OUT_HEIGHT").parse()?;
    let skip_alternating = if env_var("SKIP_ALTERNATING").parse::<u32>().unwrap() == 1 {
        true
    } else {
        false
    };
    let images = get_images(ImageSelectionConfig::from_env()?)?
        .map(|(i, _)| cropper.crop_around_middle(&i))
        .map(|i| i.resize(out_width, out_height, FilterType::Nearest));

    let f = File::create(output_fn).expect(&format!("Failed to create file {output_fn}"));
    write_gif(images, f, if skip_alternating { Fps30 } else { Fps60 })?;
    Ok(())
}

enum FrameDuration {
    Fps30,
    Fps60,
}
/// just writes the gif given the input images
fn write_gif<W: Write, I: Iterator<Item = DynamicImage>>(
    images: I,
    mut out: W,
    fd: FrameDuration,
) -> anyhow::Result<()> {
    let mut ge = GifEncoder::new_with_speed(&mut out, 1);
    ge.set_repeat(Repeat::Infinite)?;

    let (short_ms, long_ms) = match fd {
        FrameDuration::Fps30 => (20, 40),
        FrameDuration::Fps60 => (10, 20),
    };
    println!("{short_ms}, {long_ms}");
    let short_delay = Delay::from_saturating_duration(Duration::from_millis(short_ms));
    let long_delay = Delay::from_saturating_duration(Duration::from_millis(long_ms));

    ge.encode_frames(images.enumerate().map(|(c, i)| {
        let delay = if c % 2 == 0 { short_delay } else { long_delay };
        Frame::from_parts(i.into_rgba8(), 0, 0, delay)
    }))?;
    Ok(())
}

struct Cropper {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}
impl Cropper {
    fn new_from_env() -> anyhow::Result<Self> {
        let x: u32 = env_var("CROP_X").parse()?;
        let y: u32 = env_var("CROP_Y").parse()?;
        let width: u32 = env_var("CROP_WIDTH").parse()?;
        let height: u32 = env_var("CROP_HEIGHT").parse()?;
        Ok(Self {
            x,
            y,
            width,
            height,
        })
    }
    fn crop_around_middle(&self, i: &DynamicImage) -> DynamicImage {
        i.crop_imm(self.x, self.y, self.width, self.height)
    }
}
