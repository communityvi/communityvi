use crate::{connection, ConnectionIncoming, Connector, Executor, Host};
use hyper::body::HttpBody;
use hyper::service::Service;
use hyper::{Body, Request, Response, Server};
use std::future::Future;
use tokio::sync::mpsc;

pub fn serve<MakeService, HttpService, ResponseBody, MakeError, MakeFuture>(
	make_service: MakeService,
	bind_host: Option<Host>,
) -> (
	hyper::Client<Connector, Body>,
	hyper::Server<ConnectionIncoming, MakeService, Executor>,
)
where
	MakeService:
		for<'a> Service<&'a connection::Connection, Response = HttpService, Error = MakeError, Future = MakeFuture>,
	MakeError: std::error::Error + Send + Sync + 'static,
	MakeFuture: Future<Output = Result<HttpService, MakeError>> + Send + 'static,
	HttpService: Service<Request<Body>, Response = Response<ResponseBody>> + Send + 'static,
	HttpService::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
	HttpService::Future: Send,
	ResponseBody: HttpBody + Send + 'static,
	ResponseBody::Data: Send,
	ResponseBody::Error: std::error::Error + Send + Sync,
{
	let (sender, receiver) = mpsc::channel(1);
	let incoming = ConnectionIncoming::new(receiver);
	let server = Server::builder(incoming).executor(Executor).serve(make_service);

	let connector = match bind_host {
		Some(host) => Connector::new(sender).bind_to_host(host),
		None => Connector::new(sender),
	};
	let client = hyper::client::Client::builder().executor(Executor).build(connector);

	(client, server)
}
