use crate::reference_time::ReferenceTimer;
use rweb::filters::BoxedFilter;
use rweb::{get, openapi, Filter, Json, Reply};

pub fn rest_api(reference_timer: ReferenceTimer) -> BoxedFilter<(impl Reply,)> {
	let (_spec, filter) = openapi::spec().build(move || reference_time_milliseconds(reference_timer));
	filter.boxed()
}

#[get("/reference_time_milliseconds")]
fn reference_time_milliseconds(#[data] reference_timer: ReferenceTimer) -> Json<u64> {
	let milliseconds = u64::from(reference_timer.reference_time_milliseconds());
	Json::from(milliseconds)
}
