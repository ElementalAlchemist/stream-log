use super::user_info_bar::{user_bar, UserBarBuildData, UserBarClick};
use mogwai::prelude::*;
use mogwai::utils::document;
use wasm_bindgen::JsValue;

pub fn app_root() -> Option<Element> {
	document().get_element_by_id("root")
}

pub fn run_view<ClickT>(
	view: ViewBuilder<Dom>,
	user_bar_build_data: Option<UserBarBuildData<ClickT>>,
) -> Result<(), JsValue>
where
	ClickT: UserBarClick + Unpin + Send + Sync + 'static,
{
	let root_node = app_root();
	if let Some(node) = root_node {
		node.remove();
	}

	let user_bar = if let Some(user_bar_build_data) = user_bar_build_data {
		Some(user_bar(user_bar_build_data))
	} else {
		None
	};

	let full_view = view! {
		<div id="root">
			{user_bar}
			{view}
		</div>
	};

	full_view.run()
}
