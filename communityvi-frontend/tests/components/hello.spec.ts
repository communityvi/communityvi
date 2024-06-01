import Hello from '$lib/components/Hello.svelte';
import {render} from '@testing-library/svelte';
import 'vitest-dom/extend-expect';
import * as matchers from 'vitest-dom/matchers';
import {describe, it, expect} from 'vitest';

expect.extend(matchers);

describe('The Hello component', () => {
	it('inserts the passed name into the output', async () => {
		const name = 'Max';

		const renderedComponent = render(Hello, {name});
		const output = await renderedComponent.findByText('Hello Max!');

		expect(output).toHaveTextContent('Hello Max!');
	});
});
