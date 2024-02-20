/**
 * This file implements the types declared in ~/types/window.d.ts
 * as a wrapper around window.nostr methods that are commonly used
 * in react-nostr and other libraries.
 */

import { invoke } from "@tauri-apps/api/tauri";
import { nip19 } from "nostr-tools";

window.nostr = {
  getPublicKey: async function () {
    const publicKey: `npub1${string}` = await invoke("login", {});
    return nip19.decode(publicKey).data;
  },
  signEvent: async function (event: Event) {
    const signedEvent = (await invoke("sign_event", {
      event: JSON.stringify(event),
    })) as string;
    return signedEvent;
  },
};
