import Hello from '$lib/components/Hello.svelte';
import {render} from '@testing-library/svelte';
import {describe, it, expect} from 'vitest';

describe('The Hello component', () => {
	it('inserts the passed name into the output', async () => {
		const name = 'Max';

		const renderedComponent = render(Hello, {name});
		const output = await renderedComponent.findByText('Hello Max!');

		expect(output.innerHTML).toContain('Hello Max');
	});
});
