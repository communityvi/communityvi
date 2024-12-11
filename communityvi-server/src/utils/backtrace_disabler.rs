use std::ffi::OsString;

static BACKTRACE_MUTEX: parking_lot::Mutex<()> = parking_lot::const_mutex(());

pub struct BacktraceDisabler<'mutex_guard> {
	previous_environment_variable: Option<OsString>,
	_mutex_guard: parking_lot::MutexGuard<'mutex_guard, ()>,
}

const RUST_BACKTRACE: &str = "RUST_BACKTRACE";

impl Default for BacktraceDisabler<'_> {
	fn default() -> Self {
		let mutex_guard = BACKTRACE_MUTEX.lock();
		let previous_environment_variable = std::env::var_os(RUST_BACKTRACE);
		std::env::set_var(RUST_BACKTRACE, "0");
		BacktraceDisabler {
			previous_environment_variable,
			_mutex_guard: mutex_guard,
		}
	}
}

impl Drop for BacktraceDisabler<'_> {
	fn drop(&mut self) {
		if let Some(text) = self.previous_environment_variable.as_ref() {
			std::env::set_var(RUST_BACKTRACE, text);
		}
	}
}
