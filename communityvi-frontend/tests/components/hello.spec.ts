/**
 * @jest-environment jsdom
 */

import Hello from '$lib/components/Hello.svelte';
import {render} from '@testing-library/svelte';
import '@testing-library/jest-dom';

describe('The Hello component', () => {
	it('inserts the passed name into the output', async () => {
		const name = 'Max';

		const renderedComponent = render(Hello, {name});
		const output = await renderedComponent.findByText('Hello Max!');

		expect(output).toHaveTextContent('Hello Max!');
	});
});
