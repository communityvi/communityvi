import Hello from '$lib/Hello.svelte';
import {render} from '@testing-library/svelte';

describe('The Hello component', () => {
	it('inserts the passed name into the output', async () => {
		const name = 'Max';

		const renderedComponent = render(Hello, {name});
		const output = await renderedComponent.findByText('Hello Max!');

		expect(output).toHaveTextContent('Hello Max!');
	});
});

export {};
