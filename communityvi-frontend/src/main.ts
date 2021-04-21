import './styles/main.sass';

import Header from './Header.svelte';
import Main from './Main.svelte';

new Header({
	target: document.getElementsByTagName('header')[0]
});
new Main({
	target: document.getElementsByTagName('main')[0],
	props: {
		name: 'world'
	}
});
