// Flat ESLint config. Enforces the same "no escape hatches" policy as the
// Python side: no `any`, no `@ts-ignore`/`@ts-nocheck`, no `eslint-disable`
// for type rules. See CLAUDE.md.

import js from '@eslint/js';
import ts from 'typescript-eslint';
import svelte from 'eslint-plugin-svelte';
import prettier from 'eslint-config-prettier';
import globals from 'globals';

export default ts.config(
  {
    ignores: [
      'node_modules/**',
      'build/**',
      'dist/**',
      '.svelte-kit/**',
      // Claude Code subagent worktrees are full repo copies; don't lint them.
      '.claude/**',
      'src-tauri/**',
      'sidecar/**',
      'docs/**',
      // Build-time config files are excluded from the strict project; they
      // are validated by vite/svelte-kit at run time.
      'svelte.config.js',
      'vite.config.*',
      'eslint.config.js',
      '.prettierrc*',
    ],
  },

  js.configs.recommended,
  ...ts.configs.strictTypeChecked,
  ...ts.configs.stylisticTypeChecked,
  ...svelte.configs['flat/recommended'],
  prettier,
  ...svelte.configs['flat/prettier'],

  {
    languageOptions: {
      globals: { ...globals.browser, ...globals.node },
      parserOptions: {
        projectService: true,
        extraFileExtensions: ['.svelte'],
        tsconfigRootDir: import.meta.dirname,
      },
    },
    rules: {
      '@typescript-eslint/no-explicit-any': 'error',
      '@typescript-eslint/no-unsafe-argument': 'error',
      '@typescript-eslint/no-unsafe-assignment': 'error',
      '@typescript-eslint/no-unsafe-call': 'error',
      '@typescript-eslint/no-unsafe-member-access': 'error',
      '@typescript-eslint/no-unsafe-return': 'error',
      '@typescript-eslint/no-unsafe-enum-comparison': 'error',

      // No escape hatches.
      '@typescript-eslint/ban-ts-comment': [
        'error',
        {
          'ts-ignore': true,
          'ts-nocheck': true,
          'ts-check': false,
          'ts-expect-error': true,
        },
      ],
      '@typescript-eslint/no-non-null-assertion': 'error',

      // Promise rigor.
      '@typescript-eslint/no-floating-promises': 'error',
      '@typescript-eslint/no-misused-promises': 'error',
      '@typescript-eslint/require-await': 'error',

      // Style.
      '@typescript-eslint/consistent-type-imports': 'error',
      '@typescript-eslint/consistent-type-definitions': ['error', 'type'],
    },
  },

  {
    files: ['**/*.svelte'],
    languageOptions: {
      parserOptions: {
        parser: ts.parser,
        projectService: true,
        extraFileExtensions: ['.svelte'],
      },
    },
  },

  // `.svelte.ts` files contain Svelte-specific syntax (runes such as `$state`)
  // and must be parsed with the TypeScript parser. They are included in the
  // project's tsconfig via SvelteKit's generated config but ESLint needs an
  // explicit hint here.
  {
    files: ['**/*.svelte.ts'],
    languageOptions: {
      parser: ts.parser,
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
  },
);
