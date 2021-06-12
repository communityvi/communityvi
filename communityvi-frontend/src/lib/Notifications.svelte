<script lang="ts">
	import {errorBag} from '$lib/stores';
	import Notification from '$lib/Notification.svelte';
	import {NotificationType} from '$lib/notification_type';

	function deleteErrorIdentifiedBy(id: number) {
		errorBag.deleteWithId(id);
	}
</script>

{#each $errorBag as identifiableError (identifiableError.id)}
	<Notification type={NotificationType.DANGER} on:close={() => deleteErrorIdentifiedBy(identifiableError.id)}>
		{identifiableError.error.message}
	</Notification>
{/each}
