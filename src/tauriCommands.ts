import { invoke } from "@tauri-apps/api";
import { Event, listen } from "@tauri-apps/api/event";

import {
  type PayInvoiceRequestHandler,
  type PayInvoiceResponse,
  type UnsignedNostrEvent,
} from "./types";

// TODO: handle listening for getPublicKey requests

const getRandomInt = (max: number): number => {
  return Math.floor(Math.random() * max);
};

const signEventRequestHandlers: { [key: number]: SignEventRequestHandler } = {};

const payInvoiceRequestHandlers: { [key: number]: PayInvoiceRequestHandler } =
  {};

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
 * Register a handler for pay invoice requests. Any number of handlers can be registered at once.
 * When a pay invoice request is received, all registered handlers will be called one at a time.
 * If any handler returns "paid", the invoice is assumed to be paid and no further handlers will
 * be called. If all handlers have been called and none returned "paid", the outcome is assumed
 * to be "failed" if any handler returned "failed", and "denied" if all handlers returned "denied"
 * (including if no handlers are registered). Currently the order in which handlers are called is
 * unspecified.
 * @param handler The handler to register. Will be called with invoices that other apps want to pay.
 * @returns A function that can be called to unregister the handler.
 */
export const handlePayInvoiceRequests = (handler: PayInvoiceRequestHandler) => {
  // Generate a random handler ID that is not already in use.
  let handlerId = getRandomInt(1000000);
  while (payInvoiceRequestHandlers[handlerId]) {
    handlerId = getRandomInt(1000000);
  }

  payInvoiceRequestHandlers[handlerId] = handler;

  return () => {
    delete payInvoiceRequestHandlers[handlerId];
  };
};

/**
 * Get the public key of the user's Nostr account from the Tauri backend.
 * @returns The public key of the user's Nostr account.
 */
export const getPublicKey = async (): Promise<string> => {
  return await invoke("get_public_key");
};

type SignEventRequestHandler = (
  event: UnsignedNostrEvent,
) => Promise<boolean> | boolean;

listen("sign_event_request", async (event: Event<UnsignedNostrEvent>) => {
  let isApproved = false;
  for (const handler of Object.values(signEventRequestHandlers)) {
    isApproved = await handler(event.payload);
    if (isApproved) {
      break;
    }
  }
  respondToSignEventRequest(event.payload.id, isApproved);
})
  .then((unlisten) => {
    // When vite reloads, a new event listener is created, so we need to unlisten to the old one.
    // If we don't do this, each vite hot reload turns the old event listener into a phantom listener
    // that has not event handlers and therefore immediately rejects all requests.
    import.meta.hot?.on("vite:beforeUpdate", () => unlisten());
    // TODO: Send a message to the Tauri backend to indicate that the event listener is ready.
    // Before this, it should not send any `sign_event_request` events.
  })
  .catch((e) => {
    console.error(e);
  });
const respondToSignEventRequest = async (
  eventId: string,
  approved: boolean,
): Promise<string> => {
  return await invoke("respond_to_sign_event_request", { eventId, approved });
};

listen("pay_invoice_request", async (event: Event<string>) => {
  let response: PayInvoiceResponse = "rejected";
  for (const handler of Object.values(payInvoiceRequestHandlers)) {
    let newResponse = await handler(event.payload);

    // If rejected by a handler, ignore it and continue to the next handler.
    if (newResponse === "rejected") {
      continue;
    }

    response = newResponse;

    if (response === "paid") {
      break;
    }
  }
  respondToPayInvoiceRequest(event.payload, response);
})
  .then((unlisten) => {
    // When vite reloads, a new event listener is created, so we need to unlisten to the old one.
    // If we don't do this, each vite hot reload turns the old event listener into a phantom listener
    // that has not event handlers and therefore immediately rejects all requests.
    import.meta.hot?.on("vite:beforeUpdate", () => unlisten());
    // TODO: Send a message to the Tauri backend to indicate that the event listener is ready.
    // Before this, it should not send any `pay_invoice_request` events.
  })
  .catch((e) => {
    console.error(e);
  });
const respondToPayInvoiceRequest = async (
  invoice: string,
  outcome: PayInvoiceResponse,
): Promise<string> => {
  return await invoke("respond_to_pay_invoice_request", { invoice, outcome });
};
