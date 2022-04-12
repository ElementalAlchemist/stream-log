use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
	#[wasm_bindgen]
	pub fn launch_google_auth_flow(client_id: &str);
}
