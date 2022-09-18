<script lang="ts">
	import Header from '$lib/components/Header.svelte';
	import '../app.sass';
</script>

<header>
	<Header />
</header>

<main>
	<slot />
</main>
