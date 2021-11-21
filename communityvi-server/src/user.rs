use js_int::UInt;
use static_assertions::assert_obj_safe;
use user_id::UserId;

mod user_id;

pub trait User {
	fn id(&self) -> UserId;
	fn display_name(&self) -> Option<&str>;
}

assert_obj_safe!(User);

pub struct AnonymousUser {
	pub id: UInt,
	pub display_name: Option<String>,
}

impl User for AnonymousUser {
	fn id(&self) -> UserId {
		UserId::Anonymous(self.id)
	}

	fn display_name(&self) -> Option<&str> {
		self.display_name.as_deref()
	}
}
