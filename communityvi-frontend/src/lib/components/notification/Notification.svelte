<script lang="ts">
	import {fade} from 'svelte/transition';

	import {NotificationType} from '$lib/components/notification/notification_type';

	interface Properties {
		type?: NotificationType;
		useLightAppearance?: boolean;
		icon?: string;
		children?: import('svelte').Snippet;
		onClose: () => void;
	}

	let {
		type = NotificationType.PRIMARY,
		useLightAppearance = true,
		icon = 'exclamation-circle',
		children,
		onClose = () => {},
	}: Properties = $props();
</script>

<div class="notification {type}" class:is-light={useLightAppearance} out:fade>
	<button class="delete" onclick={onClose} aria-label="Close notification"></button>
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
