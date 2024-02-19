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
