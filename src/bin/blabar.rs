use image::RgbaImage;

fn main() -> anyhow::Result<()> {
    let i = RgbaImage::from_fn(1920, 1080, |_x, y| {
        if y < 20 {
            image::Rgba([0, 0, 0, 255])
        } else {
            image::Rgba([0, 0, 0, 0])
        }
    });
    i.save("blabar.png")?;
    Ok(())
}
