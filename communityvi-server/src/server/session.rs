use crate::server::session::id::SessionId;
use anyhow::bail;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
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

	pub fn start_session(&self, data: Data) -> anyhow::Result<SessionId> {
		let id = SessionId::new();
		let session = Session {
			expires_at: Instant::now() + self.inner.expire_after,
			id: SessionId::new(),
			data,
		};

		let mut sessions = self.inner.sessions.write();
		if sessions.len() >= self.inner.maximum_session_count {
			bail!("Maximum session count of {} exceeded", self.inner.maximum_session_count);
		}

		sessions.insert(id, session);

		Ok(id)
	}

	pub fn store_session(&self, mut session: Session<Data>) -> anyhow::Result<()> {
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
		let sessions = self.inner.sessions.read();
		sessions.get(&id).and_then(|session| {
			if session.has_expired() {
				None
			} else {
				Some(session.clone())
			}
		})
	}

	pub fn terminate_session(&self, id: SessionId) {
		self.inner.sessions.write().remove(&id);
	}

	fn cleanup(sessions: &mut HashMap<SessionId, Session<Data>>) {
		sessions.retain(|_id, session| !session.has_expired());
	}
}

#[derive(Clone, Debug)]
pub struct Session<Data> {
	expires_at: Instant,
	id: SessionId,
	data: Data,
}

impl<Data> Session<Data> {
	pub fn has_expired(&self) -> bool {
		Instant::now() >= self.expires_at
	}
}

impl<Data> Deref for Session<Data> {
	type Target = Data;

	fn deref(&self) -> &Self::Target {
		&self.data
	}
}

impl<Data> DerefMut for Session<Data> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.data
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

		let session1_id = session_store.start_session("session 1").unwrap();
		let session2_id = session_store.start_session("session 2").unwrap();

		let session1 = session_store.retrieve_session(session1_id).unwrap();
		let session2 = session_store.retrieve_session(session2_id).unwrap();

		assert_eq!(session1.data, "session 1");
		assert_eq!(session2.data, "session 2");
	}

	fn session_store() -> SessionStore<&'static str> {
		SessionStore::new(EXPIRE_AFTER, MAX_SESSION_COUNT)
	}
}
