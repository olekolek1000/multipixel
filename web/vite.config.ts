import child_process from 'node:child_process';

import { defineConfig } from 'vite'
import sassDts from 'vite-plugin-sass-dts'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import tsconfigPaths from 'vite-tsconfig-paths';

const connect_url = process.env.CONNECT_URL ? process.env.CONNECT_URL : "ws://127.0.0.1:59900";

let commit_hash = child_process
	.execSync('git rev-parse --short HEAD')
	.toString()
	.trim();

let commit_branch = child_process
	.execSync('git rev-parse --abbrev-ref HEAD')
	.toString()
	.trim();

// https://vite.dev/config/
export default defineConfig({
	css: {
		preprocessorOptions: {
			scss: {
				api: 'modern-compiler'
			}
		},
		modules: {
			exportGlobals: true,
		},
	},
	plugins: [
		tailwindcss(),
		tsconfigPaths(),
		react(),
		sassDts({
			enabledMode: ['development', 'production'],
			esmExport: true,
		})
	],
	define: {
		__COMMIT_HASH__: JSON.stringify(commit_hash),
		__COMMIT_BRANCH__: JSON.stringify(commit_branch),
		__CONNECT_URL__: JSON.stringify(connect_url)
	}
})
