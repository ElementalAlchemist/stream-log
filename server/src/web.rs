use super::file_types::get_media_type;
use async_std::fs;
use async_std::io::ErrorKind;
use async_std::net::TcpStream;
use async_std::path::Path;
use http_types::{Method, Response, StatusCode};
use miette::Result;

enum FileData {
	Metadata(fs::Metadata),
	Body(Vec<u8>),
}

pub async fn handle_request(stream: TcpStream) -> Result<()> {
	println!("New connection: {:?}", stream);
	if let Err(err) = async_h1::accept(stream.clone(), |req| async move {
		println!("{:?}", req);

		let get_file = match req.method() {
			Method::Get => true,
			Method::Head => false,
			_ => return Ok(Response::new(StatusCode::NotImplemented)),
		};

		let file_path = req.url().path();
		// The HTTP libraries handle relative paths that go outside the web directory.
		// Therefore, there is not special handling for those here.
		let file_path = if let Some(path_str) = file_path.strip_prefix('/') {
			path_str
		} else {
			file_path
		};
		let mut file_path = Path::new("static").join(file_path);
		if file_path.is_dir().await {
			file_path.push("index.html");
		}

		let file_result = if get_file {
			match fs::read(&file_path).await {
				Ok(contents) => Ok(FileData::Body(contents)),
				Err(err) => Err(err),
			}
		} else {
			match fs::metadata(&file_path).await {
				Ok(metadata) => Ok(FileData::Metadata(metadata)),
				Err(err) => Err(err),
			}
		};
		match file_result {
			Ok(file_data) => {
				let mut res = Response::new(StatusCode::Ok);
				res.insert_header("Content-Type", get_media_type(&file_path));
				match file_data {
					FileData::Body(data) => res.set_body(data),
					FileData::Metadata(metadata) => {
						res.insert_header("Content-Length", metadata.len().to_string());
					}
				}
				Ok(res)
			}
			Err(file_err) => match file_err.kind() {
				ErrorKind::NotFound => Ok(Response::new(StatusCode::NotFound)),
				ErrorKind::PermissionDenied => Ok(Response::new(StatusCode::Forbidden)),
				_ => Ok(Response::new(StatusCode::InternalServerError)),
			},
		}
	})
	.await
	{
		eprintln!("{}", err);
	}
	Ok(())
}
