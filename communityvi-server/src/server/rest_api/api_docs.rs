use crate::server::file_bundle::BundledFileHandler;
use rust_embed::RustEmbed;
use std::borrow::Cow;

pub fn api_docs() -> BundledFileHandler {
	#[derive(RustEmbed)]
	#[folder = "$CARGO_MANIFEST_DIR/stoplight-elements/node_modules/@stoplight/elements"]
	struct StoplightElements;

	BundledFileHandler::builder()
		.with_rust_embed::<StoplightElements>()
		.with_file(Cow::Borrowed("index.html"), INDEX_HTML)
		.build()
}

// See https://github.com/swagger-api/swagger-ui/blob/8718d4b267921b00fd616755760cc21cf4953ba9/dist/index.html
// But with modified configuration and `./` replaced with `/api/docs`
const INDEX_HTML: &str = r#"
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1, shrink-to-fit=no">
    <title>Elements in HTML</title>

    <script src="/api/docs/web-components.min.js"></script>
    <link rel="stylesheet" href="/api/docs/styles.min.css">
  </head>
  <body>

    <elements-api
      apiDescriptionUrl="/api/openapi.json"
      router="hash"
    />

  </body>
</html>
"#;
