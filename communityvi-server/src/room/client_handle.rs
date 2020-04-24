use crate::room::client::{Client, ClientId};
use dashmap::mapref::one::{Ref, RefMut};
use std::ops::Deref;

pub enum ClientHandle<'a> {
	Ref(Ref<'a, ClientId, Client>),
	RefMut(RefMut<'a, ClientId, Client>),
}

impl Deref for ClientHandle<'_> {
	type Target = Client;

	fn deref(&self) -> &Self::Target {
		match self {
			ClientHandle::Ref(reference) => reference.deref(),
			ClientHandle::RefMut(reference) => reference.deref(),
		}
	}
}

impl<'a> From<Ref<'a, ClientId, Client>> for ClientHandle<'a> {
	fn from(reference: Ref<'a, ClientId, Client>) -> Self {
		Self::Ref(reference)
	}
}

impl<'a> From<RefMut<'a, ClientId, Client>> for ClientHandle<'a> {
	fn from(reference: RefMut<'a, ClientId, Client>) -> Self {
		Self::RefMut(reference)
	}
}
