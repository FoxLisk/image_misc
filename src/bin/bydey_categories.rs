use std::{
    collections::HashMap,
    fs::{self, read_to_string, File},
    io::{BufReader, BufWriter, Cursor, Read, Write},
    marker::PhantomData,
    path::Path,
};

use ab_glyph::FontRef;
use anyhow::{anyhow, Result};
use bytes::Buf;
use image::{
    imageops::{crop, resize},
    DynamicImage, GenericImage, GenericImageView, ImageBuffer, Pixel, Rgb, Rgba,
};
use itertools::Itertools;
use regex::Regex;
use speedrun_api::{
    api::{games::GameId, variables::ValueId, Client, Query, Root},
    types::{Game, Run, Times, User, Variable},
    SpeedrunApiClient,
};

struct CategoryNameFigureOuter<'a> {
    client: SpeedrunApiClient,
    variables: HashMap<String, HashMap<String, Variable<'a>>>,
}

impl<'a> CategoryNameFigureOuter<'a> {
    fn new(client: SpeedrunApiClient) -> Self {
        Self {
            client,
            variables: Default::default(),
        }
    }

    fn get_variables(&mut self, game_id: &GameId<'_>) -> Result<HashMap<String, Variable<'_>>> {
        let gid = game_id.to_string();
        if let Some(vars) = self.variables.get(&gid) {
            return Ok(vars.clone());
        }
        let req = speedrun_api::api::games::GameVariables::builder()
            .id(game_id.to_string())
            .build()?;
        let resp: Vec<Variable> = req.query(&self.client)?;
        let vars: HashMap<String, Variable> =
            resp.into_iter().map(|v| (v.id.to_string(), v)).collect();
        let out = vars.clone();
        self.variables.insert(gid, vars);
        Ok(out)
    }

    fn get_category_name(
        &mut self,
        run: &Run<'a>,
        category: &Category,
        game: &Game<'a>,
    ) -> Result<String> {
        let mut name = category.name.clone();
        if run.values.is_empty() {
            return Ok(name);
        }
        let vars = self.get_variables(&game.id)?;
        for (var_id, value_id) in run
            .values
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .sorted()
            .map(|(a, b)| (a, ValueId::new(b)))
        {
            let whatever = var_id.to_string();
            if let Some(v) = vars.get(&whatever) {
                if v.is_subcategory {
                    let subcat_value = if let Some(v) = v.values.values.get(&value_id) {
                        v
                    } else {
                        return Err(anyhow::anyhow!(
                            "Missing subcategory variable value alfkjsdlfkj"
                        ));
                    };

                    name.push_str(&format!(" {}", subcat_value.label));
                }
            }
        }

        Ok(name)
    }
}

fn main() -> Result<()> {
    let blob = read_to_string("bydey_categories/pbs.json")?;
    let parsed: Root<Vec<PersonalBest<'_>>> = serde_json::from_str(&blob)?;

    let client = SpeedrunApiClient::new()?;
    let reqw_client = reqwest::blocking::Client::new();
    let mut cnfo = CategoryNameFigureOuter::new(client.clone());
    let bydey = get_user(&client)?;
    fs::create_dir_all("bydey_categories")?;
    fs::create_dir_all("bydey_categories/covers")?;
    fs::create_dir_all("bydey_categories/pbs")?;

    struct PB<'a> {
        game: Game<'a>,
        cat: String,
        // time: String,
        place: u32,
    }

    impl<'a> PB<'a> {
        fn from(mut game: Game<'a>, cat: String, _time: Times, place: u32) -> Self {
            Self { game, cat, place }
        }

        fn get_cover(&self, client: &reqwest::blocking::Client) -> Result<DynamicImage> {
            let pathname = format!("bydey_categories/covers/{}.png", self.game.id.to_string());
            let p = Path::new(&pathname);
            if p.exists() {
                let f = File::open(p)?;
                let br = BufReader::new(f);
                let i = image::io::Reader::new(br).with_guessed_format()?.decode()?;
                return Ok(i);
            }
            let url = self
                .game
                .assets
                .cover_medium
                .uri
                .as_ref()
                .expect(&format!(
                    "Missing cover URL for game {}",
                    self.game.names.international
                ))
                .to_string();
            let mut resp = client.get(url).send()?;
            let mut body_u8s: Vec<u8> = Vec::new();
            resp.copy_to(&mut body_u8s)?;
            let i = image::io::Reader::new(Cursor::new(body_u8s))
                .with_guessed_format()?
                .decode()?;

            i.save(p)?;
            Ok(i)
        }
    }
    let mut pbs = Vec::with_capacity(bydey.len());
    for pb_raw in bydey {
        let category_name =
            cnfo.get_category_name(&pb_raw.run, &pb_raw.category.data, &pb_raw.game.data)?;
        let pb = PB::from(
            pb_raw.game.data,
            category_name,
            pb_raw.run.times.clone(),
            pb_raw.place,
        );
        pbs.push(pb);
    }

    let font = ab_glyph::FontRef::try_from_slice(include_bytes!("../../fonts/ARIALBD.TTF"))?;

    for pb in pbs {
        let c = pb.get_cover(&reqw_client)?;

        let padded = {
            if c.width() > c.height() {
                let mut i = ImageBuffer::new(c.width(), c.width());
                let y = (c.width() - c.height()) / 2;
                i.copy_from(&c, 0, y)?;
                i
            } else if c.height() > c.width() {
                let mut i = ImageBuffer::new(c.height(), c.height());
                let x = (c.height() - c.width()) / 2;
                i.copy_from(&c, x, 0)?;
                i
            } else {
                c.to_rgba8()
            }
        };
        let resized =
            DynamicImage::ImageRgba8(padded).resize(512, 512, image::imageops::FilterType::Nearest);
        let mut nottp = resized.to_rgba8();
        for p in nottp.pixels_mut() {
            let chs = p.channels_mut();
            if chs[3] == 0 {
                chs[0] = 188;
                chs[1] = 188;
                chs[2] = 188;
            }
            p.channels_mut()[3] = 255;
        }

        let mut y_offset =
            write_potentially_long_text(&mut nottp, 0, &font, &pb.game.names.international);

        y_offset += 128;

        write_potentially_long_text(&mut nottp, y_offset, &font, &pb.cat);

        let filename = format!("{}_{}.png", pb.game.names.international, pb.cat)
            .to_lowercase()
            .replace(' ', "_")
            .replace(':', "_")
            .replace('\'', "")
            .replace('/', "_");
        let filepath = format!("bydey_categories/pbs/{filename}");

        println!("Trying to save to {filepath}");
        nottp.save(&filepath)?;
    }
    Ok(())
}

fn write_potentially_long_text(
    image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    mut y_start: i32,
    font: &FontRef,
    text: &str,
) -> i32 {
    if text.len() > 20 {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut to_write = Vec::new();
        let mut chunk = String::new();
        for word in words {
            chunk.push_str(word);
            chunk.push_str(" ");
            if chunk.len() >= 12 {
                to_write.push(chunk.clone());
                chunk.clear();
            }
        }
        if !chunk.is_empty() {
            to_write.push(chunk);
        }
        for thing in to_write {
            write_text(image, 0, y_start, &font, &thing);
            y_start += 48;
        }
        y_start
    } else {
        write_text(image, 0, y_start, &font, text);
        y_start
    }
}

fn write_text(
    image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    x: i32,
    y: i32,
    font: &FontRef,
    text: &str,
) {
    imageproc::drawing::draw_text_mut(
        image,
        Rgba::from([220, 33, 33, 255]),
        x,
        y,
        64.0,
        font,
        text,
    );
}

#[derive(serde::Deserialize, Debug)]
struct Category {
    pub id: String,
    pub name: String,
}

#[derive(serde::Deserialize, Debug)]
struct PersonalBest<'a> {
    place: u32,
    run: Run<'a>,
    game: Root<Game<'a>>,
    category: Root<Category>,
}

fn get_user(c: &SpeedrunApiClient) -> Result<Vec<PersonalBest>> {
    let req = speedrun_api::api::users::UserPersonalBests::builder()
        .id("Bydey")
        .embed(speedrun_api::api::runs::RunEmbeds::Game)
        .embed(speedrun_api::api::runs::RunEmbeds::Category)
        .build()?;

    Ok(req.query(c)?)
}
