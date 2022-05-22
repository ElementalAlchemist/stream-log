use super::dom::run_view;
use super::user_info_bar::{UserBarBuildData, UserClickTarget};
use mogwai::prelude::*;
use std::fmt::Display;

pub fn render_error_message(message: &str) {
	let error_view = builder! {
		<div class="error">
			{message}
		</div>
	};
	let user_bar_build_data: Option<UserBarBuildData<UserClickTarget>> = None;
	run_view(error_view, user_bar_build_data).expect("Failed to host view");
}

pub fn render_error_message_with_details(message: &str, error: impl Display) {
	let error_view = builder! {
		<div class="error">
			{message}
			<br />
			{error.to_string()}
		</div>
	};
	let user_bar_build_data: Option<UserBarBuildData<UserClickTarget>> = None;
	run_view(error_view, user_bar_build_data).expect("Failed to host view");
}
