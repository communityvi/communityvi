use js_int::UInt;
use user_id::UserId;

mod user_id;

trait User {
	fn id(&self) -> UserId;
	fn display_name(&self) -> Option<&str>;
}

struct AnonymousUser {
	id: UInt,
	display_name: Option<String>,
}

impl User for AnonymousUser {
	fn id(&self) -> UserId {
		UserId::Anonymous(self.id)
	}

	fn display_name(&self) -> Option<&str> {
		self.display_name.as_deref()
	}
}
