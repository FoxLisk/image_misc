use image::codecs::gif::{GifEncoder, Repeat};
use image::imageops::FilterType;
use image::{Delay, DynamicImage, Frame};
use std::cmp::Ordering;
use std::env;
use std::fs::{read_dir, File};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

fn number_from_pathbuf(p: &PathBuf) -> Option<u32> {
    p.file_stem()
        .and_then(|fs| fs.to_str())
        .and_then(|s| u32::from_str(s).ok())
}

fn compare_paths(a: &PathBuf, b: &PathBuf) -> Ordering {
    number_from_pathbuf(&a).cmp(&number_from_pathbuf(&b))
}

fn env_var(k: &str) -> String {
    env::var(k).expect(&format!("Failed to find required environment varialbe {k}"))
}

fn main() {
    dotenv::dotenv().unwrap();
    let dir = env_var("IMAGE_DIR");
    let dir_path = format!("images/{dir}");
    let out_path = format!("images/out/{dir}.gif");
    let image_paths = readdir_to_sorted(&dir_path);
    let cropper = Cropper::new_from_env();
    to_gif(image_paths, &cropper, &out_path);
}

fn readdir_to_sorted(path: &str) -> Vec<PathBuf> {
    let mut filenames = read_dir(path)
        .unwrap()
        .map(|f| f.unwrap().path())
        .collect::<Vec<PathBuf>>();
    filenames.sort_by(compare_paths);
    filenames
}

fn to_gif(filenames: Vec<PathBuf>, cropper: &Cropper, output_fn: &str) {
    let skip = env_var("SKIP_IMAGES").parse().unwrap();
    let take = env_var("TAKE_IMAGES").parse().unwrap();
    let skip_alternating = if env_var("SKIP_ALTERNATING").parse::<u32>().unwrap() == 1 {
        true
    } else {
        false
    };
    let fmap = |(c, i)| {
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
    let images = filenames
        .iter()
        .skip(skip)
        .enumerate()
        .filter_map(fmap)
        .take(take)
        .map(|f| image::open(f).unwrap())
        .map(|i| cropper.crop_around_middle(&i))
        .map(|i| i.resize(112, 112, FilterType::Nearest));

    let mut f = File::create(output_fn).unwrap();
    let mut ge = GifEncoder::new_with_speed(&mut f, 1);
    ge.set_repeat(Repeat::Infinite).unwrap();

    let (short_ms, long_ms) = if skip_alternating { (20, 40) } else { (10, 20) };
    let short_delay = Delay::from_saturating_duration(Duration::from_millis(short_ms));
    let long_delay = Delay::from_saturating_duration(Duration::from_millis(long_ms));

    ge.encode_frames(images.enumerate().map(|(c, i)| {
        let delay = if c % 2 == 0 { short_delay } else { long_delay };
        Frame::from_parts(i.into_rgba8(), 0, 0, delay)
    }))
    .unwrap();
}

struct Cropper {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}
impl Cropper {
    fn new_from_env() -> Self {
        let x: u32 = env_var("CROP_X").parse().unwrap();
        let y: u32 = env_var("CROP_Y").parse().unwrap();
        let width: u32 = env_var("CROP_WIDTH").parse().unwrap();
        let height: u32 = env_var("CROP_HEIGHT").parse().unwrap();
        Self {
            x,
            y,
            width,
            height,
        }
    }
    fn crop_around_middle(&self, i: &DynamicImage) -> DynamicImage {
        // Pixels picked kinda arbitrarily by looking at an 1172 x 896 screenshot
        i.crop_imm(self.x, self.y, self.width, self.height)
    }
}
