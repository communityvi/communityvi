use crate::reference_time::ReferenceTimer;
use rweb::filters::BoxedFilter;
use rweb::{get, openapi, router, Filter, Json, Reply};

#[cfg(feature = "api-docs")]
mod api_docs;

pub fn rest_api(reference_timer: ReferenceTimer) -> BoxedFilter<(impl Reply,)> {
	let (spec, api) = openapi::spec().build(move || api(reference_timer));
	let cors = rweb::cors().allow_any_origin().build();
	api.with(cors).or(openapi_filter(spec)).boxed()
}

pub fn openapi_filter(spec: openapi::Spec) -> BoxedFilter<(impl Reply,)> {
	let api = rweb::path("api");
	let spec_json = rweb::path("openapi.json").map(move || rweb::reply::json(&spec));
	#[cfg(not(feature = "api-docs"))]
	{
		api.and(spec_json).boxed()
	}
	#[cfg(feature = "api-docs")]
	{
		api.and(spec_json.or(rweb::path("docs").and(api_docs::api_docs())))
			.boxed()
	}
}

#[router("/api", services(reference_time_milliseconds))]
fn api(#[data] _reference_timer: ReferenceTimer) {}

#[get("/reference_time_milliseconds")]
fn reference_time_milliseconds(#[data] reference_timer: ReferenceTimer) -> Json<u64> {
	let milliseconds = u64::from(reference_timer.reference_time_milliseconds());
	Json::from(milliseconds)
}
