use axum::Router;
use reqwest::{Method, RequestBuilder};
use std::net::{Ipv6Addr, SocketAddr, SocketAddrV6};
use std::time::Duration;

pub struct TestClient {
	server_handle: axum_server::Handle,
	client: reqwest::Client,
	server_address: SocketAddr,
}

impl TestClient {
	pub async fn new_with_host(router: Router, _host: &str) -> anyhow::Result<Self> {
		// NOTE: port 0 assigns a random available port
		let socket_address = SocketAddrV6::new(Ipv6Addr::LOCALHOST, 0, 0, 0);

		let (bind_address, handle) = loop {
			let handle = axum_server::Handle::new();
			let server = axum_server::Server::bind(socket_address.into()).handle(handle.clone());

			tokio::spawn(server.serve(router.clone().into_make_service()));

			if let Some(address) = handle.listening().await {
				break (address, handle);
			}
		};

		// FIXME: Read/Write timeout
		let client = reqwest::Client::builder()
			.connect_timeout(Duration::from_secs(10))
			.build()?;

		Ok(Self {
			server_handle: handle,
			client,
			server_address: bind_address,
		})
	}

	pub fn request(&self, method: Method, path: &str) -> RequestBuilder {
		let base_address = self.server_address;
		let path = path.trim_start_matches('/');
		self.client.request(method, format!("http://{base_address}/{path}"))
	}

	pub fn get(&self, path: &str) -> RequestBuilder {
		self.request(Method::GET, path)
	}

	#[allow(unused)]
	pub fn post(&self, path: &str) -> RequestBuilder {
		self.request(Method::POST, path)
	}

	#[allow(unused)]
	pub fn put(&self, path: &str) -> RequestBuilder {
		self.request(Method::PUT, path)
	}

	#[allow(unused)]
	pub fn delete(&self, path: &str) -> RequestBuilder {
		self.request(Method::DELETE, path)
	}
}

impl Drop for TestClient {
	fn drop(&mut self) {
		self.server_handle.graceful_shutdown(Some(Duration::from_secs(5)));
	}
}
