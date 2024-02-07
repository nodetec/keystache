import { invoke } from "@tauri-apps/api";
import { listen, Event } from "@tauri-apps/api/event";

const getRandomInt = (max: number): number => {
  return Math.floor(Math.random() * max);
}

const signEventRequestHandlers: {[key: number]: SignEventRequestHandler} = {};

/**
 * Register a handler for sign event requests. Any number of handlers can be registered at once.
 * When a sign event request is received, all registered handlers will be called one at a time.
 * If any handler returns true, the event will be approved and no further handlers will be called.
 * If no handler returns true (including if no handlers are registered), the event will be denied.
 * Currently the order in which handlers are called is unspecified.
 * @param handler The handler to register. Will be called with events that other apps want to sign.
 * @returns A function that can be called to unregister the handler.
 */
export const handleSignEventRequests = (handler: SignEventRequestHandler) => {
  // Generate a random handler ID that is not already in use.
  let handlerId = getRandomInt(1000000);
  while (signEventRequestHandlers[handlerId]) {
    handlerId = getRandomInt(1000000);
  }

  signEventRequestHandlers[handlerId] = handler;

  return () => {
    delete signEventRequestHandlers[handlerId];
  };
};

/**
 * Get the public key of the user's Nostr account from the Tauri backend.
 * @returns The public key of the user's Nostr account.
 */
export const getPublicKey = async (): Promise<string> => {
  return await invoke("get_public_key");
};

interface UnsignedNostrEvent {
  id: string;
  pubkey: string;
  created_at: number;
  kind: number;
  tags: string[][];
  content: string;
};

type SignEventRequestHandler = (event: UnsignedNostrEvent) => Promise<boolean> | boolean;

listen("sign_event_request", async (event: Event<UnsignedNostrEvent>) => {
  let isApproved = false;
  for (const handler of Object.values(signEventRequestHandlers)) {
    isApproved = await handler(event.payload);
    if (isApproved) {
      break;
    }
  }
  respondToSignEventRequest(event.payload.id, isApproved);
}).then(() => {
  // TODO: Send a message to the Tauri backend to indicate that the event listener is ready.
  // Before this, it should not send any `sign_event_request` events.
});

const respondToSignEventRequest = async (eventId: string, approved: boolean): Promise<string> => {
  return await invoke("respond_to_sign_event_request", { eventId, approved });
};
