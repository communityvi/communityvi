use crate::server::session::id::SessionId;
use anyhow::bail;
use parking_lot::{RwLock, RwLockUpgradableReadGuard};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};

mod id;

#[derive(Clone)]
pub struct SessionStore<Data> {
	inner: Arc<SessionStoreInner<Data>>,
}

struct SessionStoreInner<Data> {
	sessions: RwLock<HashMap<SessionId, Session<Data>>>,
	expire_after: StdDuration,
	maximum_session_count: usize,
}

impl<Data: Clone> SessionStore<Data> {
	pub fn new(expire_after: StdDuration, maximum_session_count: usize) -> Self {
		Self {
			inner: Arc::new(SessionStoreInner {
				sessions: Default::default(),
				expire_after,
				maximum_session_count,
			}),
		}
	}

	pub fn start_session(&self, data: Data) -> anyhow::Result<Session<Data>> {
		let session = Session::new(self.inner.expire_after, data);

		let mut sessions = self.inner.sessions.write();
		if sessions.len() >= self.inner.maximum_session_count {
			// Try to make some space before bailing out.
			Self::cleanup(&mut sessions);

			// Re-check.
			if sessions.len() >= self.inner.maximum_session_count {
				bail!("Maximum session count of {} exceeded", self.inner.maximum_session_count);
			}
		}

		sessions.insert(session.id, session.clone());

		Ok(session)
	}

	fn cleanup(sessions: &mut HashMap<SessionId, Session<Data>>) {
		sessions.retain(|_id, session| !session.has_expired());
	}

	pub fn update_session(&self, mut session: Session<Data>) -> anyhow::Result<()> {
		if session.has_expired() {
			bail!("Session {} has expired.", session.id);
		}

		let mut sessions = self.inner.sessions.write();
		let old_session = if let Some(session) = sessions.get_mut(&session.id) {
			session
		} else {
			bail!("Session {} doesn't exist.", session.id);
		};

		std::mem::swap(&mut session, old_session);

		Ok(())
	}

	pub fn retrieve_session(&self, id: SessionId) -> Option<Session<Data>> {
		let sessions = self.inner.sessions.upgradable_read();
		let mut session = sessions.get(&id).and_then(|session| {
			if session.has_expired() {
				None
			} else {
				Some(session.clone())
			}
		})?;

		self.refresh_session(sessions, &mut session);

		Some(session)
	}

	fn refresh_session(
		&self,
		sessions: RwLockUpgradableReadGuard<HashMap<SessionId, Session<Data>>>,
		session: &mut Session<Data>,
	) {
		let session_time_at_least_half_expired = (session.expires_at - Instant::now()) < (self.inner.expire_after / 2);
		if session_time_at_least_half_expired {
			let sessions = &mut RwLockUpgradableReadGuard::upgrade(sessions);
			session.refresh_expires_at(self.inner.expire_after);

			sessions.insert(session.id, session.clone());
		}
	}

	pub fn terminate_session(&self, id: SessionId) {
		self.inner.sessions.write().remove(&id);
	}
}

#[derive(Clone, Debug)]
pub struct Session<Data> {
	expires_at: Instant,
	id: SessionId,
	pub data: Data,
}

impl<Data> Session<Data> {
	fn new(expire_after: StdDuration, data: Data) -> Self {
		Self {
			id: SessionId::random(),
			expires_at: Instant::now() + expire_after,
			data,
		}
	}

	pub fn id(&self) -> SessionId {
		self.id
	}

	pub fn has_expired(&self) -> bool {
		Instant::now() >= self.expires_at
	}

	fn refresh_expires_at(&mut self, expire_after: StdDuration) {
		self.expires_at = Instant::now() + expire_after;
	}
}

#[cfg(test)]
mod test {
	use super::*;

	const MAX_SESSION_COUNT: usize = 10;
	const EXPIRE_AFTER: StdDuration = StdDuration::from_secs(10);

	#[test]
	fn session_store_should_start_sessions() {
		let session_store = session_store();

		session_store.start_session("my session").expect("create session");
	}

	#[test]
	fn session_store_should_limit_number_of_sessions() {
		let session_store = session_store();

		#[allow(clippy::expect_fun_call)]
		for index in 0..MAX_SESSION_COUNT {
			session_store
				.start_session("data")
				.expect(&format!("Failed to start session {}", index));
		}

		session_store
			.start_session("data")
			.expect_err("Failed to limit number of sessions");
	}

	#[test]
	fn session_store_should_retrieve_sessions() {
		let session_store = session_store();

		let session1 = session_store.start_session("session 1").unwrap();
		let session2 = session_store.start_session("session 2").unwrap();

		let retrieved_session1 = session_store.retrieve_session(session1.id()).unwrap();
		let retrieved_session2 = session_store.retrieve_session(session2.id()).unwrap();

		assert_eq!(retrieved_session1.data, "session 1");
		assert_eq!(retrieved_session2.data, "session 2");
	}

	#[test]
	fn session_store_should_not_retrieve_expired_sessions() {
		let (session_store, expired_session) = session_store_with_session(expired_session());

		let retrieval = session_store.retrieve_session(expired_session.id);

		assert!(
			retrieval.is_none(),
			"Expired session was retrieved even though it shouldn't have been."
		);
	}

	#[test]
	fn session_store_should_clean_expired_sessions_if_no_more_sessions_are_available() {
		let session_store = SessionStore::new(EXPIRE_AFTER, 1);
		let expired_session = expired_session();
		session_store
			.inner
			.sessions
			.write()
			.insert(expired_session.id, expired_session);

		session_store
			.start_session("New session")
			.expect("Cleaning did not purge expired session.");
	}

	#[test]
	fn session_store_should_update_session_data_of_existing_sessions() {
		let session_store = session_store();
		let mut session = session_store
			.start_session("Initial state")
			.expect("Could not start session");

		session.data = "Updated state";
		session_store
			.update_session(session.clone())
			.expect("Could not update session");

		let retrieved_session = session_store
			.retrieve_session(session.id())
			.expect("Could not retrieve stored session");
		assert_eq!(retrieved_session.data, "Updated state");
	}

	#[test]
	fn session_store_should_reject_updates_to_sessions_that_do_not_exist() {
		let session_store = session_store();
		let nonexistent_session = Session::new(StdDuration::from_secs(1), "Irrelevant");

		session_store
			.update_session(nonexistent_session)
			.expect_err("Updating nonexisting sessions should not have worked, but it did.");
	}

	#[test]
	fn session_store_should_reject_updates_to_expired_sessions() {
		let (session_store, expired_session) = session_store_with_session(expired_session());

		session_store
			.update_session(expired_session)
			.expect_err("Updating an expired session has worked even though it shouldn't have.");
	}

	#[test]
	fn session_store_should_allow_session_termination() {
		let session_store = session_store();
		let session = session_store
			.start_session("Some data")
			.expect("Could not start session");

		session_store.terminate_session(session.id);

		assert!(
			session_store.retrieve_session(session.id).is_none(),
			"Session wasn't terminated."
		);
	}

	#[test]
	fn session_store_should_refresh_sessions_on_retrieval() {
		let half_expired_session = Session::new(EXPIRE_AFTER / 2 - StdDuration::from_secs(1), "Half-expired session");
		let (session_store, half_expired_session) = session_store_with_session(half_expired_session);

		let session = session_store
			.retrieve_session(half_expired_session.id)
			.expect("Could not retrieve session");

		assert!(
			session.expires_at > half_expired_session.expires_at,
			"Session wasn't refreshed."
		);
	}

	fn session_store_with_session(
		session: Session<&'static str>,
	) -> (SessionStore<&'static str>, Session<&'static str>) {
		let session_store = session_store();
		session_store.inner.sessions.write().insert(session.id, session.clone());

		(session_store, session)
	}

	fn expired_session() -> Session<&'static str> {
		Session {
			expires_at: Instant::now() - StdDuration::from_secs(1),
			id: SessionId::random(),
			data: "Expired",
		}
	}

	fn session_store() -> SessionStore<&'static str> {
		SessionStore::new(EXPIRE_AFTER, MAX_SESSION_COUNT)
	}
}
