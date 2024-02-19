import { Buffer } from "buffer";

import { useEffect, useState } from "react";

import { SatoshiV2Icon } from "@bitcoin-design/bitcoin-icons-react/filled";
import useStore from "~/store";
import { UnsignedNostrEvent } from "~/types";
import { bech32 } from "bech32";
import { type Event, type EventTemplate } from "nostr-tools";
import {
  allTags,
  finishEvent,
  InvoiceResponse,
  profileContent,
  RelayUrl,
  tag,
  useBatchedProfiles,
  usePublish,
  useZap,
  ZapResponseBody,
} from "react-nostr";
import QRCode from "react-qr-code";

import { Button } from "../ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "../ui/dialog";
import { Label } from "../ui/label";
import { Textarea } from "../ui/textarea";

type Props = {
  handleOpen?: (bool: boolean) => void;
  open: boolean;
  event: UnsignedNostrEvent;
};

export default function QRCodeModal({
  handleOpen,
  // event,
  // amount,
  event,
  open,
}: Props) {
  // const { subRelays, pubRelays } = useRelayStore();
  // TODO: Relays should come from event
  // const subRelays: `wss://${string}`[] = ["wss://nos.lol"];
  // const pubRelays: `wss://${string}`[] = ["wss://nos.lol"];
  // const {zap, status} = useZap()
  const { pubkey } = useStore();
  const [message, setMessage] = useState("");

  const relays = event.tags
    ?.filter((tag) => tag[0] === "relays")
    .map((tag) => tag.slice(1))
    .flat() as RelayUrl[];
  console.log("relays", relays);

  console.log("event", event);

  const amount = tag("amount", event as Event);

  const [invoice, setInvoice] = useState("");

  const { zap, status: zapStatus } = useZap({
    // eventKey: `zap-${profileEvent.id}`,
    eventKey: "zap",
    // relays: subRelays,
    relays,
  });

  const { publish, removeEvent, addEvent, status } = usePublish({
    // relays: pubRelays,
    relays,
  });

  const profileEvent = useBatchedProfiles(tag("p", event as Event)!, relays);

  useEffect(() => {
    async function onOpenChange() {
      if (open) {
        console.log("doin onOpenChange!");
        const recipientPubkey = event.pubkey;
        const tags = [];

        let lnurl = "";
        const { lud16 } = profileContent(profileEvent);
        // const lud16 = "chrisatmachine@getalby.com";
        console.log("profileEvent", profileEvent);
        if (lud16) {
          const [name, domain] = lud16.split("@");
          lnurl = `https://${domain}/.well-known/lnurlp/${name}`;
        } else {
          return;
        }
        const res = await fetch(lnurl);
        const body = (await res.json()) as ZapResponseBody;
        console.log("zap body!", body);
        let callback;
        if (body.allowsNostr && body.nostrPubkey) {
          callback = body.callback;
        } else {
          console.log("zap body doesnt allow nostry or have a pubkey!");
          return;
        }

        const lnurlEncoded = bech32.encode(
          "lnurl",
          bech32.toWords(Buffer.from(lnurl, "utf8")),
          2000,
        );

        console.log("lud16", profileContent(event as Event).lud16!);

        console.log("lnurlEncoded", lnurlEncoded);

        // if (subRelays[0]) {
        //   tags.push(["e", applicationEvent.id, subRelays[0]]);
        // } else {
        //   tags.push(["e", applicationEvent.id]);
        // }

        const amountInMilisats = (
          parseInt(amount ? amount : "0") * 1000
        ).toString();
        // const amountInMilisats = "10000";
        console.log("amountInMilisats", amountInMilisats);
        console.log("amount", amount);

        // tags.push(["amount", amountInMilisats]);
        // tags.push(["lnurl", lnurlEncoded]);
        // tags.push(["p", recipientPubkey]);
        // tags.push(["relays", ...relays]);

        // const eventTemplate = {
        //   kind: 9734,
        //   tags,
        //   content: "", // this can be a comment
        //   created_at: Math.floor(Date.now() / 1000),
        //   pubkey,
        // };

        // const zapRequestEvent = await finishEvent(eventTemplate);
        // console.log("zapRequestEvent", zapRequestEvent);
        // console.log(
        //   `${callback}?amount=${amountInMilisats}&nostr=${encodeURIComponent(JSON.stringify(zapRequestEvent))}&lnurl=${lnurlEncoded}`,
        // );
        const invoiceRes = await fetch(
          `${callback}?amount=${amountInMilisats}&nostr=${encodeURIComponent(JSON.stringify(event))}&lnurl=${lnurlEncoded}`,
        );

        const { pr: invoice } = (await invoiceRes.json()) as InvoiceResponse;

        console.log("invoice", invoice);

        if (invoice) {
          setInvoice(invoice);
        }

        console.log("invoice!", invoice);

        // const signedEvent = await finishEvent(eventTemplate);
        const onSuccess = (event: Event) => {
          console.log("event!", event);
        };

        // await publish(signedEvent, onSuccess);
      }
    }

    onOpenChange().catch(console.error);
  }, [open]);

  return (
    <Dialog open={open} onOpenChange={handleOpen}>
      <DialogContent className="sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle>Pay Invoice</DialogTitle>
          <DialogDescription>
            Scan this QR code with a lightning enabled wallet to pay the
            invoice.
          </DialogDescription>
        </DialogHeader>
        <div className="flex flex-col gap-y-2">
          <div className="m-auto">
            <QRCode value={invoice} />
            <Label className=" flex items-center py-4 text-left text-base font-semibold">
              Amount:
              <span className="flex items-center text-orange-500 dark:text-orange-400">
                <SatoshiV2Icon className="h-5 w-5" />
                {Number(tag("amount", event as Event)).toLocaleString()}
              </span>
            </Label>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
