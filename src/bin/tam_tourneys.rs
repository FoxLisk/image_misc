use std::fs::create_dir_all;

use ab_glyph::FontRef;
use anyhow::anyhow;
use image::Rgb;
use imageproc::drawing::text_size;

fn main() -> anyhow::Result<()> {
    let tourneys = vec![
        ("2024", "No Logic"),
        ("2023", "Casual Boots"),
        ("2023", "HMG"),
        ("2023", "Main Tourney"),
        ("2023", "Casual Boots XKeys"),
        ("2022", "No Logic"),
        ("2022", "Main Tourney"),
        ("2022", "AD Keys"),
        ("2021", "HMG"),
        ("2021", "XKeys"),
        ("2021", "Main Tourney"),
        ("2020", "SGL"),
        ("2020", "Co-op"),
        ("Season 1", "NMG League"),
        ("Season 2", "NMG League"),
        ("Season 5", "NMG League"),
        ("Season 6", "NMG League"),
    ];

    let backgrounds = vec![[128 + 32, 64 + 32, 128 + 64], [128 + 64, 64 + 32, 128 + 32]];
    let mut bg_iter = backgrounds.iter().cycle();

    let font = ab_glyph::FontRef::try_from_slice(include_bytes!("../../fonts/ARIALBD.TTF"))?;

    create_dir_all("tam_tourneys")?;
    for tourney in tourneys {
        let fname = format!("tam_tourneys/{}_{}.png", tourney.1, tourney.0);
        let blah = draw_image(tourney, bg_iter.next().unwrap(), &font)?;
        blah.save(fname)?;
    }
    Ok(())
}

fn draw_image(
    tourney: (&str, &str),
    bg: &[u8; 3],
    font: &FontRef,
) -> anyhow::Result<image::RgbImage> {
    let (when, name) = tourney;
    let size = 512;
    let mut i = image::RgbImage::new(size, size);
    for p in i.pixels_mut() {
        p.0 = bg.clone();
    }
    let when_scale = 112.0;
    let (when_x, _) = text_size(when_scale, font, when);
    imageproc::drawing::draw_text_mut(
        &mut i,
        Rgb::from([30, 60, 30]),
        ((size / 2) - (when_x / 2)) as i32,
        16,
        when_scale,
        font,
        when,
    );

    let mut y_offset = size / 8;

    for line in name.split(' ').rev() {
        let name_scale = 112.0;
        let (x, y) = text_size(name_scale, font, line);
        if x > size {
            return Err(anyhow!("`{line}` too long"));
        }
        imageproc::drawing::draw_text_mut(
            &mut i,
            Rgb::from([30, 60, 30]),
            ((size - x) / 2) as i32,
            (size - y - y_offset) as i32,
            name_scale,
            font,
            line,
        );
        y_offset += y;
    }

    Ok(i)
}
