use async_std::path::Path;

/// Gets the media type (MIME type) for a file based on the file path.
/// Uses "application/octet-stream" as the type for files with an unknown type.
pub fn get_media_type(file_path: &Path) -> &str {
	let file_extension = if let Some(ext) = file_path.extension() {
		ext
	} else {
		return "application/octet-stream";
	};
	let file_extension = if let Some(ext) = file_extension.to_str() {
		ext
	} else {
		return "application/octet-stream";
	};
	match file_extension {
		"html" => "text/html",
		"css" => "text/css",
		"js" => "text/javascript",
		"wasm" => "application/wasm",
		_ => "application/octet-stream",
	}
}
