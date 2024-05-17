import { useEffect, useRef, useState } from "react";

import Profile from "~/components/profile/Profile";
import { Button } from "~/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "~/components/ui/dialog";
import QRCodeModalTrigger from "~/components/zaps/QRCodeModalTrigger";
import {
  getPublicKey,
  handlePayInvoiceRequests,
  handleSignEventRequests,
} from "~/tauriCommands";
import { type PayInvoiceResponse, type UnsignedNostrEvent } from "~/types";

const HomePage = () => {
  const [open, setOpen] = useState(false);
  const [event, setEvent] = useState<UnsignedNostrEvent | undefined>(undefined);
  const [invoice, setInvoice] = useState<string | undefined>(undefined);
  const [pubkey, setPubkey] = useState<string | undefined | null>(undefined);
  const resolveRejectRef = useRef<{
    resolve: (value: boolean) => void;
    reject: (value: boolean) => void;
  } | null>(null);

  const resolveRejectInvoiceRef = useRef<{
    resolve: (value: PayInvoiceResponse) => void;
    reject: (value: PayInvoiceResponse) => void;
  } | null>(null);

  async function setPublicKey() {
    const response = await getPublicKey();
    setPubkey(response);
  }

  // Reject the request if the dialog is closed.
  // Otherwise, the request will be pending forever.
  useEffect(() => {
    if (!open) {
      handleReject();
    }
  }, [open]);

  useEffect(() => {
    setPublicKey();
  }, []);

  useEffect(() => {
    const handleEvent = (event: UnsignedNostrEvent): Promise<boolean> => {
      setEvent(event);
      setOpen(true);
      return new Promise((resolve, reject) => {
        resolveRejectRef.current = { resolve, reject };
      });
    };

    return handleSignEventRequests(handleEvent);
  }, []);

  useEffect(() => {
    const handleEvent = (invoice: string): Promise<PayInvoiceResponse> => {
      setInvoice(invoice);
      setOpen(true);
      return new Promise((resolve, reject) => {
        resolveRejectInvoiceRef.current = { resolve, reject };
      });
    };

    return handlePayInvoiceRequests(handleEvent);
  });

  const handleAccept = () => {
    if (resolveRejectRef.current) {
      resolveRejectRef.current.resolve(true);
    }
    setOpen(false);
  };

  const handleReject = () => {
    if (resolveRejectRef.current) {
      resolveRejectRef.current.resolve(false);
    }
    setOpen(false);
  };

  return (
    <>
      <div className="pt-4">
        <h1 className=" text-2xl font-semibold tracking-tight">
          Signed in as:{" "}
        </h1>
        {pubkey && (
          <Profile
            pubkey={pubkey}
            filter={{
              authors: [pubkey],
              kinds: [0],
              limit: 1,
            }}
          />
        )}
        {invoice && <QRCodeModalTrigger invoice={invoice} />}
      </div>

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="h-[20rem] max-w-[22rem]">
          <DialogHeader>
            <DialogTitle>Sign Event?</DialogTitle>
          </DialogHeader>
          <div className="overflow-auto bg-muted">
            <pre>{JSON.stringify(event, null, 2) ?? ""}</pre>
          </div>
          <DialogFooter>
            <div className="flex justify-end gap-x-4">
              <Button onClick={handleAccept}>Accept</Button>
              <Button variant="outline" onClick={handleReject}>
                Reject
              </Button>
            </div>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
};

export default HomePage;
