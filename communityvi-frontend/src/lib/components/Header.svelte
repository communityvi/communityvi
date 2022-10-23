<script lang="ts">
	import {page} from '$app/stores';
	import Notifications from '$lib/components/Notifications.svelte';

	let isBurgerMenuExpanded = false;

	function toggleBurgerMenu() {
		isBurgerMenuExpanded = !isBurgerMenuExpanded;
	}
</script>

<nav class="navbar" aria-label="main navigation">
	<div class="navbar-brand">
		<a class="navbar-item is-teaser" href="/">
			<span>Communityvi</span>
		</a>

		<span
			role="button"
			class="navbar-burger"
			class:is-active={isBurgerMenuExpanded}
			on:click={toggleBurgerMenu}
			on:keypress={event => {
				if (event.key === 'Enter') {
					toggleBurgerMenu();
				}
			}}
			aria-label="menu"
			aria-expanded={isBurgerMenuExpanded}
		>
			<span aria-hidden="true" />
			<span aria-hidden="true" />
			<span aria-hidden="true" />
			<span aria-hidden="true" />
		</span>
	</div>

	<div class="navbar-menu" class:is-active={isBurgerMenuExpanded}>
		<div class="navbar-start">
			<a class="navbar-item" class:is-active={$page.url.pathname === '/about'} href="/about">About</a>
		</div>
	</div>
</nav>

<Notifications />

<style lang="sass">
	.is-teaser
		font-weight: bold

		&:hover
			color: unset
</style>
