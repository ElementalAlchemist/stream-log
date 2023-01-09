use contrast::contrast;
use rgb::RGB8;
use std::error::Error;
use std::fmt;
use std::num::ParseIntError;
use sycamore::prelude::*;

const LIGHT_BACKGROUND: RGB8 = RGB8::new(255, 255, 255);
const DARK_BACKGROUND: RGB8 = RGB8::new(16, 16, 16);

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

#[derive(Prop)]
pub struct ColorInputProps<'a, 'b> {
	color: &'a Signal<String>,
	username: &'a ReadSignal<String>,
	view_id: &'b str,
}

#[component]
pub fn ColorInputWithContrast<'a, 'b, G: Html>(ctx: Scope<'a>, props: ColorInputProps<'a, 'b>) -> View<G> {
	let light_color_contrast_signal = create_memo(ctx, || {
		let color = color_from_rgb_str(&props.color.get()).unwrap_or_else(|_| RGB8::new(127, 127, 127));
		let color_contrast: f64 = contrast(color, LIGHT_BACKGROUND);
		color_contrast
	});
	let dark_color_contrast_signal = create_memo(ctx, || {
		let color = color_from_rgb_str(&props.color.get()).unwrap_or_else(|_| RGB8::new(127, 127, 127));
		let color_contrast: f64 = contrast(color, DARK_BACKGROUND);
		color_contrast
	});

	let input_id = format!("{}_color_input", props.view_id);
	let input_id_for = input_id.clone();
	view! {
		ctx,
		div {
			div {
				label(for=input_id_for) {
					"Color: "
				}
				input(id=input_id, type="color", bind:value=props.color)
			}
			div(class="color_input_preview") {
				div(class="color_input_preview_light", style=format!("color: {}", *props.color.get())) { (*props.username.get()) }
				div(class="color_input_preview_light_contrast") { "Contrast: " (*light_color_contrast_signal.get()) }
				div(class="color_input_preview_dark", style=format!("color: {}", *props.color.get())) { (*props.username.get()) }
				div(class="color_input_preview_dark_contrast") { "Contrast: " (*dark_color_contrast_signal.get()) }
			}
		}
	}
}
