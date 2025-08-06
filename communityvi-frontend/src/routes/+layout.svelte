<script lang="ts">
	import Header from '$lib/components/Header.svelte';
	import '../app.sass';
	interface Properties {
		children?: import('svelte').Snippet;
	}

	let { children }: Properties = $props();
</script>

<header>
	<Header />
</header>

<main>
	{@render children?.()}
</main>
