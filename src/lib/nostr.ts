import { invoke } from "@tauri-apps/api/tauri";
import { nip19 } from "nostr-tools";

interface Window {
  nostr: Nostr;
}

// https://github.com/nostr-protocol/nips/blob/master/07.md
interface Nostr {
  getPublicKey(): Promise<unknown>;
  signEvent(event: unknown): Promise<unknown>;
}

declare const window: Window;
declare const nostr: Nostr;

window.nostr = {
  getPublicKey: async function () {
    const publicKey: `npub1${string}` = await invoke("login", {});
    return nip19.decode(publicKey).data;
  },
  signEvent: async function (event: Event) {
    const signedEvent: string = (await invoke("sign_event", {
      event: JSON.stringify(event),
    }));
    return signedEvent;
  },
};
