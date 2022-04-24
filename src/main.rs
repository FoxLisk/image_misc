use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, DynamicImage, Frame, GenericImageView};
use std::cmp::Ordering;
use std::fs::{read_dir, File};
use std::io::Result;
use std::path::PathBuf;
use std::str::FromStr;

fn number_from_pathbuf(p: &PathBuf) -> Option<u32> {
    p.file_stem()
        .and_then(|fs| fs.to_str())
        .and_then(|s| u32::from_str(s).ok())
}

fn compare_paths(a: &PathBuf, b: &PathBuf) -> Ordering {
    number_from_pathbuf(&a).cmp(&number_from_pathbuf(&b))
}

fn main() {
    let enter_filenames = readdir_to_sorted("images/noeg_enter");
    to_gif(enter_filenames, "images/out/noeg_enter.gif");

    let leave_filenames = readdir_to_sorted("images/noeg_leave");
    to_gif(leave_filenames, "images/out/noeg_leave.gif");
}

fn readdir_to_sorted(path: &str) -> Vec<PathBuf> {

    let mut filenames = read_dir(path)
        .unwrap()
        .map(|f| f.unwrap().path())
        .collect::<Vec<PathBuf>>();
    filenames.sort_by(compare_paths);
    filenames
}

fn to_gif(filenames: Vec<PathBuf>, output_fn: &str) {
    let images = filenames.iter().map(|f| image::open(f).unwrap());

    let cropped = images.map(|i| crop_around_middle(&i));

    let mut f = File::create(output_fn).unwrap();
    let mut ge = GifEncoder::new_with_speed(&mut f, 1);
    ge.set_repeat(Repeat::Infinite);
    ge.encode_frames(cropped.enumerate().filter_map(|(c, i)| {
        if c % 2 == 0 {
            Some(Frame::from_parts(
                i.into_rgba8(),
                0,
                0,
                Delay::from_numer_denom_ms(1000, 15),
            ))
        } else {
            None
        }
    }))
        .unwrap();
}

fn crop_around_middle(i: &DynamicImage) -> DynamicImage {
    // Pixels picked kinda arbitrarily by looking at an 1172 x 896 screenshot
    i.crop_imm(518, 342, 160, 160)
}
