/**
 * This file declares the types for the window.nostr method wrapper,
 * which is implemented in ~/lib/nostr.ts
 */
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
