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
import useStore from "~/store";
import { handleSignEventRequests } from "~/tauriCommands";
import { type UnsignedNostrEvent } from "~/types";

const HomePage = () => {
  const [open, setOpen] = useState(false);
  const [event, setEvent] = useState<UnsignedNostrEvent | undefined>(undefined);
  const resolveRejectRef = useRef<{
    resolve: (value: boolean) => void;
    reject: (value: boolean) => void;
  } | null>(null);

  const { pubkey } = useStore();

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
