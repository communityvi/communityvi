const {pathsToModuleNameMapper} = require('ts-jest/utils');
const {compilerOptions} = require('./tsconfig');

module.exports = {
	globals: {
		'ts-jest': {
			tsconfig: './tsconfig.json',
		},
	},
	transform: {
		'^.+\\.svelte$': [
			'svelte-jester',
			{
				preprocess: true,
			},
		],
		'^.+\\.ts$': 'ts-jest',
	},
	setupFilesAfterEnv: ['<rootDir>/tests/jest-setup.ts'],
	moduleFileExtensions: ['ts', 'js', 'svelte'],
	moduleNameMapper: pathsToModuleNameMapper(compilerOptions.paths, {prefix: '<rootDir>/'}),
};
