<script context="module" lang="ts">
	export const prerender = true;
</script>

<script lang="ts">
	import Fa from 'svelte-fa/src/fa.svelte';
	import {faHouseUser} from '@fortawesome/free-solid-svg-icons';
	import Hello from '$lib/Hello.svelte';
</script>

<svelte:head>
	<title>Home</title>
</svelte:head>

<section>
	<Fa icon={faHouseUser} />
	<Hello name="User" />
</section>
