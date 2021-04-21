<script lang="ts">
	let isBurgerMenuExpanded = false;

	function toggleBurgerMenu() {
		isBurgerMenuExpanded = !isBurgerMenuExpanded;
	}
</script>

<nav class="navbar" role="navigation" aria-label="main navigation">
	<div class="navbar-brand">
		<a class="navbar-item is-teaser" href="/#">
			<span>Communityvi</span>
		</a>

		<a
				role="button"
				class="navbar-burger"
				class:is-active={isBurgerMenuExpanded}
				on:click={toggleBurgerMenu}
				aria-label="menu"
				aria-expanded={isBurgerMenuExpanded}
				href="/#"
		>
			<span aria-hidden="true"></span>
			<span aria-hidden="true"></span>
			<span aria-hidden="true"></span>
			<span aria-hidden="true"></span>
		</a>
	</div>

	<div class="navbar-menu" class:is-active={isBurgerMenuExpanded}>
		<div class="navbar-start">
			<a class="navbar-item" href="/#">Home</a>
			<a class="navbar-item" href="/#">About</a>
			<a class="navbar-item" href="/#">Downloads</a>
		</div>
	</div>
</nav>

<style type="text/sass">
	.is-teaser
		font-weight: bold

		&:hover
			color: unset
</style>
