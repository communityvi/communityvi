use crate::room::client::{Client, ClientId};
use dashmap::mapref::one::{Ref, RefMut};
use std::ops::Deref;

pub enum ClientReference<'a> {
	Ref(Ref<'a, ClientId, Client>),
	RefMut(RefMut<'a, ClientId, Client>),
}

impl Deref for ClientReference<'_> {
	type Target = Client;

	fn deref(&self) -> &Self::Target {
		match self {
			ClientReference::Ref(reference) => reference.deref(),
			ClientReference::RefMut(reference) => reference.deref(),
		}
	}
}

impl<'a> From<Ref<'a, ClientId, Client>> for ClientReference<'a> {
	fn from(reference: Ref<'a, ClientId, Client>) -> Self {
		Self::Ref(reference)
	}
}

impl<'a> From<RefMut<'a, ClientId, Client>> for ClientReference<'a> {
	fn from(reference: RefMut<'a, ClientId, Client>) -> Self {
		Self::RefMut(reference)
	}
}
