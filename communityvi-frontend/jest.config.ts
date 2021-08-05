import {pathsToModuleNameMapper} from 'ts-jest/utils';
import {compilerOptions} from './tsconfig.json';

export default {
	globals: {
		'ts-jest': {
			tsconfig: './tsconfig.json',
			isolatedModules: true,
			useESM: true,
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
	moduleNameMapper: pathsToModuleNameMapper(compilerOptions.paths, {prefix: '<rootDir>/'}),
	moduleFileExtensions: ['ts', 'js', 'svelte'],
	extensionsToTreatAsEsm: ['.ts', '.svelte'],
	testEnvironment: 'node',
};
