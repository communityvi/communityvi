<script lang="ts">
	import Header from '$lib/components/Header.svelte';
	import '../app.sass';
	interface Props {
		children?: import('svelte').Snippet;
	}

	let { children }: Props = $props();
</script>

<header>
	<Header />
</header>

<main>
	{@render children?.()}
</main>
