import { vi } from "vitest";

// The shell supplies these platform boundaries; retained tests run real DOM/editor transforms.
vi.mock("@tauri-apps/plugin-os", () => ({ type: () => "linux" }));
vi.mock("@tauri-apps/api/core", async importOriginal => ({
	...(await importOriginal<typeof import("@tauri-apps/api/core")>()),
	convertFileSrc: (path: string, protocol = "asset") =>
		`http://${protocol}.localhost/${encodeURIComponent(path)}`,
}));
