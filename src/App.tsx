import { useEffect, useState, useRef } from "react";
import { handleSignEventRequests } from "./tauriCommands";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "./components/ui/dialog";
import { Button } from "./components/ui/button";

const App = () => {
  const [publicKey, setPublicKey] = useState("");
  const [open, setOpen] = useState(false);
  const [event, setEvent] = useState({});
  const resolveRejectRef = useRef<{
    resolve: (value: boolean) => void;
    reject: (value: boolean) => void;
  } | null>(null);

  useEffect(() => {
    const handleEvent = (event: any): Promise<boolean> => {
      setEvent(event);
      setOpen(true);
      return new Promise((resolve, reject) => {
        resolveRejectRef.current = { resolve, reject };
      });
    };

    handleSignEventRequests(handleEvent);
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
      <div className="container">
        <h1>Welcome to Keystache!</h1>
      </div>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="h-[20rem] max-w-[22rem]">
          <DialogHeader>
            <DialogTitle>Sign Event?</DialogTitle>
          </DialogHeader>
          <div className="overflow-auto bg-muted">
            <pre>{JSON.stringify(event, null, 2)}</pre>
          </div>
          <DialogFooter>
            <div className="flex justify-end gap-x-4">
              <Button onClick={handleAccept}>Accept</Button>
              <Button variant="outline" onClick={handleReject}>Reject</Button>
            </div>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
};

export default App;
