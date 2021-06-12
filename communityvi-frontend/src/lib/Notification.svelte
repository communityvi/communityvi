<script lang="ts">
	import {createEventDispatcher} from 'svelte';
	import {fade} from 'svelte/transition';

	import {NotificationType} from '$lib/notification_type';

	export let type: NotificationType = NotificationType.PRIMARY;
	export let useLightAppearance = true;
	export let icon = 'exclamation-circle';

	const dispatch = createEventDispatcher();
</script>

<div class="notification {type}" class:is-light={useLightAppearance} out:fade>
	<button class="delete" on:click={() => dispatch('close')} />
	<div class="media">
		<div class="media-left">
			<span class="icon">
				<i class="fas fa-{icon}" />
			</span>
		</div>

		<div class="media-content">
			<slot />
		</div>
	</div>
</div>
