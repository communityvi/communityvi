#![allow(clippy::needless_pass_by_value)] // #[data] requires owned values
use crate::room::Room;
use rweb::filters::BoxedFilter;
use rweb::{get, openapi, Filter, Json, Reply};

pub fn rest_api(room: Room) -> BoxedFilter<(impl Reply,)> {
	let (_spec, filter) = openapi::spec().build(move || reference_time_milliseconds(room));
	filter.boxed()
}

#[get("/reference_time_milliseconds")]
fn reference_time_milliseconds(#[data] room: Room) -> Json<u64> {
	#[allow(clippy::cast_possible_truncation)]
	let milliseconds = room.current_reference_time().as_millis() as u64;
	Json::from(milliseconds)
}
