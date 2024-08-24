use web_sys::window;

pub fn set_page_title(new_title: &str) {
	if let Some(window) = window() {
		if let Some(document) = window.document() {
			document.set_title(new_title);
		}
	}
}
