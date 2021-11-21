use js_int::UInt;
use std::fmt::{Display, Formatter};

pub enum UserId {
	Anonymous(UInt),
	Named(String),
}

impl Display for UserId {
	fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
		match self {
			UserId::Anonymous(id) => write!(formatter, "Anonymous#{}", id),
			UserId::Named(name) => formatter.write_str(name),
		}
	}
}
