use crate::server::api::Sessions;
use crate::server::session::{id::SessionId, SessionStore};
use crate::user::{AnonymousUser, User};
use rweb::{get, path, post, router, Filter, Rejection, Reply};
use std::sync::Arc;

// FIXME: Clean this up, this is only a proof of concept

pub fn sessions(sessions: Sessions) -> impl Clone + Filter<Extract = (impl Reply,), Error = Rejection> {
	create_session(sessions.clone()).or(get_session(sessions))
}

#[post("/sessions")]
pub fn create_session(#[data] sessions: Sessions, #[body] user: String) -> String {
	let session = sessions.start_session(user).unwrap();
	session.id().to_string()
}

#[get("/sessions/{session_id}")]
pub fn get_session(#[data] sessions: Sessions, session_id: String) -> String {
	let session_id = SessionId::try_from(session_id.as_str()).unwrap();
	let session = if let Some(session) = sessions.retrieve_session(session_id) {
		session
	} else {
		return "Not Found".into();
	};

	session.data
}
