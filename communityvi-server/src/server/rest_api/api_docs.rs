use crate::server::file_bundle::{BundledFile, BundledFileHandler};
use rust_embed::RustEmbed;
use rweb::filters::BoxedFilter;
use rweb::Filter;
use rweb::Reply;
use std::borrow::Cow;

pub fn api_docs() -> BoxedFilter<(impl Reply,)> {
	#[derive(RustEmbed)]
	#[folder = "$CARGO_MANIFEST_DIR/swagger-ui/node_modules/swagger-ui-dist"]
	struct SwaggerUi;

	BundledFileHandler::builder()
		.with_rust_embed::<SwaggerUi>()
		.with_file(BundledFile::new(Cow::Borrowed("index.html"), INDEX_HTML.as_bytes()))
		.build()
		.into_rweb_filter()
		.boxed()
}

// See https://github.com/swagger-api/swagger-ui/blob/8718d4b267921b00fd616755760cc21cf4953ba9/dist/index.html
// But with modified configuration and `./` replaced with `/api/docs`
const INDEX_HTML: &str = r#"
<!-- HTML for static distribution bundle build -->
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8">
    <title>Swagger UI</title>
    <link rel="stylesheet" type="text/css" href="/api/docs/swagger-ui.css" />
    <link rel="icon" type="image/png" href="/api/docs/favicon-32x32.png" sizes="32x32" />
    <link rel="icon" type="image/png" href="/api/docs/favicon-16x16.png" sizes="16x16" />
    <style>
      html
      {
        box-sizing: border-box;
        overflow: -moz-scrollbars-vertical;
        overflow-y: scroll;
      }

      *,
      *:before,
      *:after
      {
        box-sizing: inherit;
      }

      body
      {
        margin:0;
        background: #fafafa;
      }
    </style>
  </head>

  <body>
    <div id="swagger-ui"></div>

    <script src="/api/docs/swagger-ui-bundle.js" charset="UTF-8"> </script>
    <script src="/api/docs/swagger-ui-standalone-preset.js" charset="UTF-8"> </script>
    <script>
    window.onload = function() {
      // Begin Swagger UI call region
      const ui = SwaggerUIBundle({
        url: "/api/openapi.json",
        dom_id: '#swagger-ui',
        deepLinking: true,
        presets: [
          SwaggerUIBundle.presets.apis,
          SwaggerUIStandalonePreset
        ],
        plugins: [
          SwaggerUIBundle.plugins.DownloadUrl
        ],
        layout: "StandaloneLayout"
      });
      // End Swagger UI call region

      window.ui = ui;
    };
  </script>
  </body>
</html>
"#;
