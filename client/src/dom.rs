use mogwai::prelude::*;
use mogwai::utils::document;
use wasm_bindgen::JsValue;

pub fn app_root() -> Element {
	document().get_element_by_id("root").unwrap()
}

pub fn run_view(view: View<Dom>) -> Result<(), JsValue> {
	let root_node = app_root();
	while let Some(child) = root_node.first_element_child() {
		child.remove();
	}
	view.run_in_container(&root_node)
}
