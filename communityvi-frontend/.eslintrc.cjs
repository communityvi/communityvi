module.exports = {
	root: true,
	parser: '@typescript-eslint/parser',
	extends: ['eslint:recommended', 'plugin:@typescript-eslint/recommended', 'prettier'],
	plugins: ['svelte3', '@typescript-eslint'],
	ignorePatterns: ['*.cjs'],
	overrides: [{files: ['*.svelte'], processor: 'svelte3/svelte3'}],
	settings: {
		'svelte3/typescript': () => require('typescript'),
		'svelte3/ignore-styles': () => true, // WHY??!!?! Svelte eslint plugin requires a function to return a bool...
	},
	parserOptions: {
		sourceType: 'module',
		ecmaVersion: 2020,
	},
	env: {
		browser: true,
		es2017: true,
		node: true,
	},
	rules: {
		indent: ['error', 'tab', {SwitchCase: 1}],
		'linebreak-style': ['error', 'unix'],
		quotes: ['error', 'single'],
		semi: ['error', 'always'],
	},
};
