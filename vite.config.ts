/// <reference types="vite/client" />

import { defineConfig } from "vitest/config";
import react, { reactCompilerPreset } from "@vitejs/plugin-react";
import babel from "@rolldown/plugin-babel";

const host = process.env.TAURI_DEV_HOST as string | null;

// https://vitejs.dev/config/
export default defineConfig({
	plugins: [
		react(),
		babel({
			presets: [reactCompilerPreset()],
		}),
	],

	// Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
	//
	// 1. prevent vite from obscuring rust errors
	clearScreen: false,
	// 2. tauri expects a fixed port, fail if that port is not available
	server: {
		port: 1420,
		strictPort: true,
		host: host ?? false,
		hmr: host
			? {
					protocol: "ws",
					host,
					port: 1421,
				}
			: undefined,
		watch: {
			// 3. tell vite to ignore watching `src-tauri`
			ignored: ["**/src-tauri/**"],
		},
	},

	test: {
		globals: true,
		environment: "jsdom",
		setupFiles: "./src/__test__/setup.ts",
		clearMocks: true,
	},
});
