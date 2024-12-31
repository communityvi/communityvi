import Hello from '$lib/components/Hello.svelte';
import {render} from '@testing-library/svelte';
import {describe, it, expect} from 'vitest';

describe('The Hello component', () => {
	it('inserts the passed name into the output', async () => {
		const name = 'Max';

		const renderedComponent = render(Hello, {name});
		const outputHtml = (await renderedComponent.findByText('Hello Max!')).innerHTML;

		expect(outputHtml).toContain('Hello Max!');
	});
});
