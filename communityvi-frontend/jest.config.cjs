const {pathsToModuleNameMapper} = require('ts-jest');
const {compilerOptions} = require('./tsconfig');

module.exports = {
	transform: {
		'^.+\\.svelte$': [
			'svelte-jester',
			{
				preprocess: true,
			},
		],
		'^.+\\.ts$': [
			'ts-jest',
			{
				tsconfig: './tsconfig.json',
			},
		],
	},
	testEnvironment: 'jsdom',
	moduleFileExtensions: ['ts', 'js', 'svelte'],
	moduleNameMapper: pathsToModuleNameMapper(compilerOptions.paths, {prefix: '<rootDir>/'}),
};
