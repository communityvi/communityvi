use crate::connection::split_websocket;
use crate::context::ApplicationContext;
use crate::lifecycle::run_client;
use crate::room::Room;
use crate::server::unwind_safe_gotham_handler::UnwindSafeGothamHandler;
use gotham::hyper::http::{HeaderMap, Response};
use gotham::hyper::upgrade::OnUpgrade;
use gotham::hyper::Body;
use gotham::hyper::StatusCode;
use gotham::router::builder::{build_simple_router, DefineSingleRoute, DrawRoutes, RouterBuilder};
use gotham::router::Router;
use gotham::state::{FromState, State};
use log::error;

#[cfg(feature = "bundle-frontend")]
mod frontend;
mod unwind_safe_gotham_handler;
mod websocket_upgrade;

pub type WebSocket = tokio_tungstenite::WebSocketStream<gotham::hyper::upgrade::Upgraded>;

pub async fn run_server(application_context: &ApplicationContext) {
	let room = Room::new(application_context.configuration.room_size_limit);
	let _ = gotham::init_server(
		application_context.configuration.address,
		create_router(application_context.clone(), room),
	)
	.await;
}

pub fn create_router(application_context: ApplicationContext, room: Room) -> Router {
	build_simple_router(move |route| {
		route.get("/ws").to_new_handler(UnwindSafeGothamHandler::from({
			let application_context = application_context.clone();
			move |state| websocket_handler(application_context, room, state)
		}));
		add_frontend_handler(route);
	})
}

#[cfg(feature = "bundle-frontend")]
fn add_frontend_handler(route: &mut RouterBuilder<(), ()>) {
	route.get("/*").to(frontend::frontend_handler);
}

#[cfg(not(feature = "bundle-frontend"))]
fn add_frontend_handler(_route: &mut RouterBuilder<(), ()>) {}

fn websocket_handler(application_context: ApplicationContext, room: Room, mut state: State) -> (State, Response<Body>) {
	let headers = HeaderMap::take_from(&mut state);
	let on_upgrade = OnUpgrade::try_take_from(&mut state);
	let response = match on_upgrade {
		Some(on_upgrade) if websocket_upgrade::requested(&headers) => {
			match websocket_upgrade::accept(&headers, on_upgrade) {
				Ok((response, websocket_future)) => {
					tokio::spawn(async move {
						match websocket_future.await {
							Ok(websocket) => run_client_connection(application_context, room, websocket).await,
							Err(error) => error!("Failed to upgrade websocket with error {:?}.", error),
						}
					});
					response
				}
				Err(()) => bad_request(),
			}
		}
		_ => bad_request(),
	};
	(state, response)
}

fn bad_request() -> Response<Body> {
	Response::builder()
		.status(StatusCode::BAD_REQUEST)
		.body(Body::from("Bad Request"))
		.expect("Failed to build BAD_REQUEST response.")
}

async fn run_client_connection(application_context: ApplicationContext, room: Room, websocket: WebSocket) {
	let (message_sender, message_receiver) = split_websocket(websocket);
	run_client(application_context, room, message_sender, message_receiver).await;
}
