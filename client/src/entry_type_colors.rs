// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use contrast::contrast;
use rgb::RGB8;

pub const WHITE: RGB8 = RGB8::new(255, 255, 255);
pub const BLACK: RGB8 = RGB8::new(0, 0, 0);

pub fn use_white_foreground(color: &RGB8) -> bool {
	let white_contrast: f64 = contrast(*color, WHITE);
	let black_contrast: f64 = contrast(*color, BLACK);

	white_contrast > black_contrast
}
