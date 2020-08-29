use gotham::handler::{Handler, HandlerFuture, IntoHandlerFuture, NewHandler};
use gotham::state::State;
use std::panic::AssertUnwindSafe;
use tokio::macros::support::Pin;

/// Helper type to provide a way around the `Copy` and `UnwindSafe` bounds required for routing to
/// gotham `Handler`s.
/// This provides `Handler` for closures and `NewHandler` as well if they implement `Clone`.
pub struct UnwindSafeGothamHandler<HandlerType> {
	handler: AssertUnwindSafe<HandlerType>,
}

impl<Closure, IntoHandler> From<Closure> for UnwindSafeGothamHandler<Closure>
where
	Closure: FnOnce(State) -> IntoHandler + Send,
	IntoHandler: IntoHandlerFuture,
{
	fn from(handler: Closure) -> Self {
		UnwindSafeGothamHandler {
			handler: AssertUnwindSafe(handler),
		}
	}
}

impl<Closure, IntoHandler> Handler for UnwindSafeGothamHandler<Closure>
where
	Closure: FnOnce(State) -> IntoHandler + Send,
	IntoHandler: IntoHandlerFuture,
{
	fn handle(self, state: State) -> Pin<Box<HandlerFuture>> {
		let closure = self.handler.0;
		closure(state).into_handler_future()
	}
}

impl<Closure, IntoHandler> NewHandler for UnwindSafeGothamHandler<Closure>
where
	Closure: FnOnce(State) -> IntoHandler + Send + Sync + Clone,
	IntoHandler: IntoHandlerFuture,
{
	type Instance = Self;

	fn new_handler(&self) -> anyhow::Result<Self::Instance> {
		let closure = self.handler.0.clone();
		Ok(UnwindSafeGothamHandler {
			handler: AssertUnwindSafe(closure),
		})
	}
}
