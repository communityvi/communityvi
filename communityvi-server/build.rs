use npm_rs::NpmEnv;
use std::env;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let frontend_path = Path::new("../communityvi-frontend");
	match env::var("CARGO_FEATURE_BUNDLE_FRONTEND") {
		Ok(_) => {
			if is_debug_profile() {
				rerun_if_frontend_changes(frontend_path);
			}

			let exit_status = NpmEnv::default()
				.set_path(frontend_path)
				.init_env()
				.install(None)
				.run("build")
				.exec()?;
			if !exit_status.success() {
				return Err("Npm build failed".into());
			}
		}
		Err(_) => {
			// don't always rerun build.rs
			println!("cargo:rerun-if-changed=build.rs")
		}
	}
	Ok(())
}

fn is_debug_profile() -> bool {
	env::var("PROFILE") == Ok("debug".to_string())
}

// Prints the necessary cargo directives for rebuilding only
// if changes to the frontend directory have been detected.
fn rerun_if_frontend_changes(frontend_path: &'static Path) {
	let package_lock_json_path = frontend_path.join("package-lock.json");

	for result in ignore::WalkBuilder::new(frontend_path)
		.hidden(false)
		.ignore(false)
		.require_git(false)
		.build()
	{
		let entry = result.unwrap();
		let path = entry.path();

		// These paths are touched by npm, so ignore them
		if (path == package_lock_json_path) || (path == frontend_path) {
			continue;
		}

		println!("cargo:rerun-if-changed={}", path.display());
	}
}
