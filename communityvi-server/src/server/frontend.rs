use gotham::hyper::header::{CONTENT_LENGTH, CONTENT_TYPE};
use gotham::hyper::{Body, Response, StatusCode, Uri};
use gotham::state::State;
use include_dir::{include_dir, Dir};
use mime_guess::MimeGuess;

const FRONTEND_BUILD: Dir = include_dir!("../communityvi-frontend/build");

pub fn frontend_handler(state: State) -> (State, Response<Body>) {
	let uri = state.borrow::<Uri>();
	let path = uri.path().strip_prefix("/").unwrap_or(uri.path());
	// TODO: Implement ETAG and cache headers
	match FRONTEND_BUILD.get_file(path) {
		Some(file) => {
			let mime = MimeGuess::from_path(path).first_or_octet_stream();
			let content = file.contents();
			(
				state,
				Response::builder()
					.status(StatusCode::OK)
					.header(CONTENT_TYPE, mime.as_ref())
					.header(CONTENT_LENGTH, content.len())
					.body(Body::from(content))
					.unwrap(),
			)
		}
		None => (state, not_found()),
	}
}

fn not_found() -> Response<Body> {
	Response::builder()
		.status(StatusCode::NOT_FOUND)
		.body(Body::from("Not found"))
		.expect("Failed to build NOT_FOUND response")
}
