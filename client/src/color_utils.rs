use rgb::RGB8;
use std::error::Error;
use std::fmt;
use std::num::ParseIntError;

pub const LIGHT_BACKGROUND: RGB8 = RGB8::new(255, 255, 255);
pub const DARK_BACKGROUND: RGB8 = RGB8::new(16, 16, 16);

#[derive(Debug)]
pub enum RgbColorError {
	InvalidLength,
	InvalidData(ParseIntError),
}

impl fmt::Display for RgbColorError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::InvalidLength => write!(f, "color value is an invalid length"),
			Self::InvalidData(error) => write!(f, "could not parse color value: {}", error),
		}
	}
}

impl Error for RgbColorError {}

impl From<ParseIntError> for RgbColorError {
	fn from(value: ParseIntError) -> Self {
		Self::InvalidData(value)
	}
}

/// Converts a color from a #abcdef string to an RGB8.
pub fn color_from_rgb_str(color_str: &str) -> Result<RGB8, RgbColorError> {
	let color_str = if let Some(s) = color_str.strip_prefix('#') {
		s
	} else {
		color_str
	};

	let mut color_red = String::with_capacity(2);
	let mut color_green = String::with_capacity(2);
	let mut color_blue = String::with_capacity(2);

	let mut color_chars = color_str.chars();

	color_red.push(if let Some(c) = color_chars.next() {
		c
	} else {
		return Err(RgbColorError::InvalidLength);
	});
	color_red.push(if let Some(c) = color_chars.next() {
		c
	} else {
		return Err(RgbColorError::InvalidLength);
	});
	color_green.push(if let Some(c) = color_chars.next() {
		c
	} else {
		return Err(RgbColorError::InvalidLength);
	});
	color_green.push(if let Some(c) = color_chars.next() {
		c
	} else {
		return Err(RgbColorError::InvalidLength);
	});
	color_blue.push(if let Some(c) = color_chars.next() {
		c
	} else {
		return Err(RgbColorError::InvalidLength);
	});
	color_blue.push(if let Some(c) = color_chars.next() {
		c
	} else {
		return Err(RgbColorError::InvalidLength);
	});
	if color_chars.next().is_some() {
		return Err(RgbColorError::InvalidLength);
	}

	let color_red = u8::from_str_radix(&color_red, 16)?;
	let color_green = u8::from_str_radix(&color_green, 16)?;
	let color_blue = u8::from_str_radix(&color_blue, 16)?;

	Ok(RGB8::new(color_red, color_green, color_blue))
}

pub fn rgb_str_from_color(color: RGB8) -> String {
	format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
}
