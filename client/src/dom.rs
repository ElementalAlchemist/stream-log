use super::user_info_bar::{user_bar, ClickTarget, SuppressibleUserBarParts};
use mogwai::prelude::*;
use mogwai::utils::document;
use stream_log_shared::messages::user::UserData;
use wasm_bindgen::JsValue;

pub struct ViewData {
	pub user_bar_click_channel: Option<mpsc::Receiver<ClickTarget>>,
}

pub fn app_root() -> Option<Element> {
	document().get_element_by_id("root")
}

pub fn run_view(
	view: ViewBuilder<Dom>,
	user: Option<&UserData>,
	suppress_user_bar_parts: &[SuppressibleUserBarParts],
) -> Result<ViewData, JsValue> {
	let root_node = app_root();
	if let Some(node) = root_node {
		node.remove();
	}

	let (user_bar, user_bar_click_channel) = if let Some(user) = user {
		let user_bar_data = user_bar(user, suppress_user_bar_parts);
		(Some(user_bar_data.view), Some(user_bar_data.click_channel))
	} else {
		(None, None)
	};

	let full_view = view! {
		<div id="root">
			{user_bar}
			{view}
		</div>
	};

	full_view.run()?;

	Ok(ViewData { user_bar_click_channel })
}
