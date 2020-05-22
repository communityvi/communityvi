use std::ffi::OsString;

static BACKTRACE_MUTEX: parking_lot::Mutex<()> = parking_lot::const_mutex(());

pub struct BacktraceDisabler<'mutex_guard> {
	previous_environment_variable: Option<OsString>,
	_mutex_guard: parking_lot::MutexGuard<'mutex_guard, ()>,
}

const RUST_BACKTRACE: &str = "RUST_BACKTRACE";

impl<'mutex_guard> Default for BacktraceDisabler<'mutex_guard> {
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

impl<'mutex_guard> Drop for BacktraceDisabler<'mutex_guard> {
	fn drop(&mut self) {
		self.previous_environment_variable
			.as_ref()
			.map(|text| std::env::set_var(RUST_BACKTRACE, text));
	}
}
