use contrast::contrast;
use rgb::RGB8;

pub const WHITE: RGB8 = RGB8::new(255, 255, 255);
pub const BLACK: RGB8 = RGB8::new(0, 0, 0);

pub fn use_white_foreground(color: &RGB8) -> bool {
	let white_contrast: f64 = contrast(*color, WHITE);
	let black_contrast: f64 = contrast(*color, BLACK);

	white_contrast > black_contrast
}
