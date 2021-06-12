<script lang="ts">
	import {Notification} from 'svelma';
	import {errorBag} from '$lib/stores';

	function deleteErrorIdentifiedBy(id: number) {
		errorBag.deleteWithId(id);
	}
</script>

{#each $errorBag as identifiableError (identifiableError.id)}
	<Notification type="is-danger" on:close={() => deleteErrorIdentifiedBy(identifiableError.id)}>
		{identifiableError.error.message}
	</Notification>
{/each}
