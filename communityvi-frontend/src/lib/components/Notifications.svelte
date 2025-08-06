<script lang="ts">
	import {notifications} from '$lib/stores';
	import Notification from '$lib/components/notification/Notification.svelte';

	function deleteNotificationIdentifiedBy(id: number) {
		notifications.deleteWithId(id);
	}
</script>

{#each $notifications as notification (notification.id)}
	<Notification type={notification.type} onClose={() => deleteNotificationIdentifiedBy(notification.id)}>
		{notification.message}
	</Notification>
{/each}
