<script lang="ts">
	interface Props {
		name: string;
	}

	let { name }: Props = $props();
</script>

<div id="main">
	<h1>Hello {name}!</h1>
</div>
