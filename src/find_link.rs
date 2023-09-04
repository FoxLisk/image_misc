use image::{DynamicImage, GenericImageView, ImageBuffer, Pixel};
use std::collections::HashSet;
use std::fmt::Display;

const HAT_COLOR: [u8; 3] = [123, 189, 33];

pub fn find_link(image: &DynamicImage) -> Option<(i32, i32)> {
    let mut possible_links_hat_pixels: HashSet<(i32, i32)> = Default::default();

    for (x, y, px) in image.pixels() {
        let rgb = px.to_rgb();
        if rgb.0 == HAT_COLOR {
            possible_links_hat_pixels.insert((x as i32, y as i32));
        }
    }
    let matchers = make_facing_matchers();
    for matcher in matchers.iter() {
        if let Some(link_pos) = matcher.find_link_topleft(&possible_links_hat_pixels) {
            return Some(link_pos);
        }
    }
    None
}

fn find_match(pixels: &HashSet<(i32, i32)>, reqd_offsets: &Vec<(i32, i32)>) -> Option<(i32, i32)> {
    let _is_match_starting_from = |(start_x, start_y): &(i32, i32)| {
        for (offset_x, offset_y) in reqd_offsets {
            let new_x = start_x + offset_x;
            let new_y = start_y + offset_y;
            let new_p = (new_x, new_y);
            if !pixels.contains(&new_p) {
                return false;
            }
        }
        true
    };

    for pixel in pixels.iter() {
        if _is_match_starting_from(pixel) {
            return Some(pixel.clone());
        }
    }
    None
}

/// looks for link by checking the pixels for ones that match his hat colour.
/// the specific patterns such as the sort of r-ish shape while he's facing up are checked
/// against the pixels
///
/// expects standard 256x224 images. might be wrong in some cases
struct FacingMatcher {
    name: String,
    hat_px_offsets: Vec<(i32, i32)>,
    link_offset_from_hat_found_location: (i32, i32),
}

impl FacingMatcher {
    fn new<S: Display>(
        name: S,
        hat_px_offsets: Vec<(i32, i32)>,
        link_offset_from_hat_found_location: (i32, i32),
    ) -> Self {
        Self {
            name: name.to_string(),
            hat_px_offsets,
            link_offset_from_hat_found_location,
        }
    }

    fn find_link_topleft(
        &self,
        possible_links_hat_pixels: &HashSet<(i32, i32)>,
    ) -> Option<(i32, i32)> {
        find_match(possible_links_hat_pixels, &self.hat_px_offsets).map(|(x, y)| {
            let (xo, yo) = &self.link_offset_from_hat_found_location;
            (x + xo, y + yo)
        })
    }
}

fn make_facing_matchers() -> Vec<FacingMatcher> {
    let mut matchers = vec![
        FacingMatcher::new(
            "facing_up_hat_top",
            vec![(1, 0), (2, 0), (3, 0), (4, 0), (-1, 1), (0, 1), (1, 1)],
            (-6, -2),
        ),
        // this is only checking the wort of backwards P shape because the hook covers the top part
        FacingMatcher::new(
            "facing_up_hat_bottom",
            vec![(1, 0), (0, 1), (1, 1), (1, 2), (1, 3), (1, 4)],
            (-9, -7),
        ),
        FacingMatcher::new(
            "facing_right_hat_top",
            vec![(1, 0), (-1, 1), (0, 1), (1, 1), (-2, 2), (-1, 2), (0, 2)],
            (-7, -1),
        ),
        FacingMatcher::new(
            "facing_right_hat_jiggling",
            vec![(5, 0), (6, 0), (6, -1), (0, 1), (1, 1), (4, 1), (5, 1)],
            (-2, -2),
        ),
        FacingMatcher::new(
            "right_facing_pot_pickup",
            vec![
                (1, 0),
                (6, 0),
                (7, 0),
                (1, 1),
                (6, 1),
                (7, 1),
                (8, 1),
                (7, 2),
                (8, 2),
                (8, 3),
            ],
            (-3, -1),
        ),
    ];
    matchers
}
