import App from '../src/App.svelte';
import { render } from '@testing-library/svelte'

describe('The main App component', () => {
	it('inserts the passed name into the output', async () => {
		const name = 'Max';

		const renderedComponent = render(App, { name });
		const output = await renderedComponent.findByText('Hello Max!');

		expect(output).toHaveTextContent('Hello Max!');
	});
});
