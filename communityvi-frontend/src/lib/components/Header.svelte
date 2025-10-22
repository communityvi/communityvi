<script lang="ts">
	import {resolve} from '$app/paths';
	import {page} from '$app/state';
	import Notifications from '$lib/components/Notifications.svelte';

	let isBurgerMenuExpanded = $state(false);

	function toggleBurgerMenu() {
		isBurgerMenuExpanded = !isBurgerMenuExpanded;
	}
</script>

<nav class="navbar" aria-label="main navigation">
	<div class="navbar-brand">
		<a class="navbar-item is-teaser" href={resolve('/')}>
			<span>Communityvi</span>
		</a>

		<span
			role="button"
			class="navbar-burger"
			class:is-active={isBurgerMenuExpanded}
			onclick={toggleBurgerMenu}
			onkeydown={event => {
				if (event.key === 'Enter') {
					toggleBurgerMenu();
				}
			}}
			tabindex="0"
			aria-label="menu"
			aria-expanded={isBurgerMenuExpanded}
		>
			<span aria-hidden="true"></span>
			<span aria-hidden="true"></span>
			<span aria-hidden="true"></span>
			<span aria-hidden="true"></span>
		</span>
	</div>

	<div class="navbar-menu" class:is-active={isBurgerMenuExpanded}>
		<div class="navbar-start">
			<a class="navbar-item" class:is-active={page.url.pathname === '/about'} href={resolve('/about')}>About</a>
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
