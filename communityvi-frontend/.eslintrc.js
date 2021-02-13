module.exports = {
	'env': {
		'browser': true,
		'es2021': true
	},
	'extends': [
		'eslint:recommended',
		'plugin:@typescript-eslint/recommended',
		'plugin:@typescript-eslint/recommended-requiring-type-checking'
	],
	'parser': '@typescript-eslint/parser',
	'parserOptions': {
		'ecmaVersion': 12,
		'sourceType': 'module',
		'tsconfigRootDir': __dirname,
		'project': ['./tsconfig.json'],
		'extraFileExtensions': ['.svelte'],
	},
	'plugins': [
		'svelte3',
		'@typescript-eslint',
	],
	'overrides': [
		{
			'files': ['*.svelte'],
			'processor': 'svelte3/svelte3'
		}
	],
	'rules': {
		'indent': [
			'error',
			'tab'
		],
		'linebreak-style': [
			'error',
			'unix'
		],
		'quotes': [
			'error',
			'single'
		],
		'semi': [
			'error',
			'always'
		]
	},
	'settings': {
		'svelte3/ignore-styles': () => true, // WHY??!!?! Svelte eslint plugin requires a function to return a bool...
		'svelte3/typescript': require('typescript'), // Dependency Injection via settings? Sure!
	}
};
