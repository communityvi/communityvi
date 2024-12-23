<script lang="ts">
	import {createEventDispatcher} from 'svelte';
	import {fade} from 'svelte/transition';

	import {NotificationType} from '$lib/components/notification/notification_type';

	interface Props {
		type?: NotificationType;
		useLightAppearance?: boolean;
		icon?: string;
		children?: import('svelte').Snippet;
	}

	let {
		type = NotificationType.PRIMARY,
		useLightAppearance = true,
		icon = 'exclamation-circle',
		children
	}: Props = $props();

	const dispatch = createEventDispatcher();
</script>

<div class="notification {type}" class:is-light={useLightAppearance} out:fade>
	<button class="delete" onclick={() => dispatch('close')} aria-label="Close notification"></button>
	<div class="media">
		<div class="media-left">
			<span class="icon">
				<i class="fas fa-{icon}"></i>
			</span>
		</div>

		<div class="media-content">
			{@render children?.()}
		</div>
	</div>
</div>
