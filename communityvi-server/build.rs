use ignore::DirEntry;
use npm_rs::NpmEnv;
use std::env;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let should_bundle_frontend = env::var("CARGO_FEATURE_BUNDLE_FRONTEND").is_ok();
	let should_bundle_stoplight_elements = env::var("CARGO_FEATURE_API_DOCS").is_ok();
	if should_bundle_frontend {
		bundle_frontend()?;
	}

	if should_bundle_stoplight_elements {
		bundle_stoplight_elements()?;
	}

	if !should_bundle_frontend && !should_bundle_stoplight_elements {
		// don't always rerun build.rs
		println!("cargo:rerun-if-changed=build.rs")
	}

	Ok(())
}

fn bundle_frontend() -> Result<(), Box<dyn std::error::Error>> {
	let frontend_path = Path::new("../communityvi-frontend");

	if is_debug_profile() {
		limit_rerun_to_frontend_changes(frontend_path);
	}

	let exit_status = NpmEnv::default()
		.set_path(frontend_path)
		.init_env()
		.install(None)
		.run("build")
		.exec()?;
	if !exit_status.success() {
		return Err("Npm build of frontend failed".into());
	}

	Ok(())
}

fn bundle_stoplight_elements() -> Result<(), Box<dyn std::error::Error>> {
	let stoplight_elements_path = Path::new("stoplight-elements");

	if is_debug_profile() {
		limit_rerun_to_frontend_changes(stoplight_elements_path);
	}

	let exit_status = NpmEnv::default()
		.set_path(stoplight_elements_path)
		.init_env()
		.install(None)
		.exec()?;
	if !exit_status.success() {
		return Err("Npm install of stoplight elements failed".into());
	}

	Ok(())
}

fn is_debug_profile() -> bool {
	env::var("PROFILE") == Ok("debug".to_string())
}

// Prints the necessary cargo directives for rebuilding only
// if changes to the frontend directory have been detected.
fn limit_rerun_to_frontend_changes(frontend_path: &Path) {
	let package_lock_json_path = frontend_path.join("package-lock.json");

	for entry in files_and_directories_not_in_gitignore(frontend_path) {
		let path = entry.path();

		// These paths are changed by npm when building the frontend
		if (path == package_lock_json_path) || (path == frontend_path) {
			continue;
		}

		println!("cargo:rerun-if-changed={}", path.display());
	}
}

fn files_and_directories_not_in_gitignore(path: &Path) -> impl Iterator<Item = DirEntry> {
	ignore::WalkBuilder::new(path)
		.hidden(false)
		.ignore(false)
		.require_git(false)
		.build()
		.map(Result::unwrap)
}
