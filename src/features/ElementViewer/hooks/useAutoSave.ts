import { useCallback, useEffect, useId, useRef } from "react";
import { notifications } from "@mantine/notifications";
import { AUTO_SAVE_DELAY_IN_MILLISECONDS } from "../../../config/constants";
import {
	defaultGlobalSyncEventManager,
	ListenerType,
} from "../../../stores/sync/managers/syncEventManager";
import { defaultCloseRequestedEventManager } from "../../../managers/closeRequestedEventManager";
import errorToString from "../../../utils/errorToString";

interface PendingSave {
	read: () => string | Promise<string>;
	save: (content: string) => Promise<void>;
}
const pending = new Map<string, PendingSave>();
let running: Promise<void> | null = null;
let timeout: ReturnType<typeof setTimeout>;
let registered = false;

// Pending writes outlive editors, so failed unmounts can still retry on sync/close.
export function flushPendingSaves(): Promise<void> {
	clearTimeout(timeout);
	if (running) return running;
	running = (async () => {
		while (pending.size) {
			const failures: unknown[] = [];
			for (const [id, entry] of [...pending]) {
				try {
					await entry.save(await entry.read());
					if (pending.get(id) === entry) pending.delete(id);
				} catch (error) {
					failures.push(error);
				}
			}
			if (failures.length) throw failures[0];
		}
	})().finally(() => {
		running = null;
	});
	return running;
}

function saveInBackground() {
	void flushPendingSaves().catch(error =>
		notifications.show({
			message: `Changes are still unsaved: ${errorToString(error)}`,
			color: "red",
		}),
	);
}

export default function useAutoSave(
	onSave: (content: string) => Promise<void>,
	initial?: string,
) {
	const id = useId();
	const acknowledged = useRef(initial);
	const saveRef = useRef(onSave);
	useEffect(() => {
		saveRef.current = onSave;
	}, [onSave]);
	useEffect(() => {
		if (!registered) {
			registered = true;
			defaultCloseRequestedEventManager.addHandler(
				"save pending content",
				{ cb: flushPendingSaves, priority: 0 },
			);
			defaultGlobalSyncEventManager.addListener(
				ListenerType.PreSyncStart,
				flushPendingSaves,
			);
			window.addEventListener("beforeunload", event => {
				if (pending.size || running) event.preventDefault();
				saveInBackground();
			});
		}
		return saveInBackground;
	}, []);

	return useCallback(
		(read: () => string | Promise<string>) => {
			const save = saveRef.current;
			pending.set(id, {
				read,
				save: async content => {
					if (content === acknowledged.current) return;
					await save(content);
					acknowledged.current = content;
				},
			});
			clearTimeout(timeout);
			timeout = setTimeout(
				saveInBackground,
				AUTO_SAVE_DELAY_IN_MILLISECONDS,
			);
		},
		[id],
	);
}
