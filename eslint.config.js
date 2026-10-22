import js from '@eslint/js';
import globals from 'globals';
import reactHooks from 'eslint-plugin-react-hooks';
import reactRefresh from 'eslint-plugin-react-refresh';
import tseslint from 'typescript-eslint';

export default tseslint.config(
  { ignores: ['dist', 'node_modules', 'extension/build'] },
  { extends: [js.configs.recommended, ...tseslint.configs.recommended], languageOptions: { globals: globals.browser } },
  { files: ['extension/**/*.js'], languageOptions: { globals: { chrome: 'readonly', importScripts: 'readonly' } } },
  { plugins: { 'react-hooks': reactHooks, 'react-refresh': reactRefresh }, rules: { ...reactHooks.configs.recommended.rules, ...reactRefresh.configs.vite.rules } }
);
