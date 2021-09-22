use npm_rs::NpmEnv;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	match env::var("CARGO_FEATURE_BUNDLE_FRONTEND") {
		Ok(_) => {
			let exit_status = NpmEnv::default()
				.set_path("../communityvi-frontend")
				.init_env()
				.install(None)
				.run("build")
				.exec()?;
			if !exit_status.success() {
				return Err("Npm build failed".into());
			}
		}
		Err(_) => {
			println!("cargo:rerun-if-changed=build.rs")
		}
	}
	Ok(())
}
