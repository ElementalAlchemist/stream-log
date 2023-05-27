use crate::color_utils::{color_from_rgb_str, DARK_BACKGROUND, LIGHT_BACKGROUND};
use contrast::contrast;
use rgb::RGB8;
use sycamore::prelude::*;

#[derive(Prop)]
pub struct ColorInputProps<'a, 'b> {
	color: &'a Signal<String>,
	username: &'a ReadSignal<String>,
	view_id: &'b str,
}

#[component]
pub fn ColorInputWithContrast<'a, G: Html>(ctx: Scope<'a>, props: ColorInputProps<'a, '_>) -> View<G> {
	let light_color_contrast_signal = create_memo(ctx, || {
		let color = color_from_rgb_str(&props.color.get()).unwrap_or_else(|_| RGB8::new(127, 127, 127));
		let color_contrast: f64 = contrast(color, LIGHT_BACKGROUND);
		format!("{:.4}", color_contrast)
	});
	let dark_color_contrast_signal = create_memo(ctx, || {
		let color = color_from_rgb_str(&props.color.get()).unwrap_or_else(|_| RGB8::new(127, 127, 127));
		let color_contrast: f64 = contrast(color, DARK_BACKGROUND);
		format!("{:.4}", color_contrast)
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
